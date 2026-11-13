# The browser demo as a static site: the page from `demo/`, livre compiled to
# wasm32 with its wasm-bindgen glue in `pkg/`, the test PDFs in `fixtures/`,
# and a `version.json` the footer reads.
{
  pkgs,
  craneLib,
  # The cargo sources, from `source.nix`.
  src,
  # The commit the site is built from, shown in the footer.
  rev,
}: let
  inherit (pkgs) lib;
  fs = lib.fileset;
  root = ../.;

  inherit (craneLib.crateNameFromCargoToml {cargoToml = root + "/crates/livre/Cargo.toml";}) version;

  wasmArgs = {
    inherit src version;
    pname = "livre-wasm";
    strictDeps = true;
    CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
    cargoExtraArgs = "--locked -p livre-wasm";
    doCheck = false;
  };

  # Its own `cargoArtifacts`: the `livre-wasm32` check only type-checks, so its
  # dependency artifacts hold no code to link.
  wasm = craneLib.buildPackage (
    wasmArgs
    // {
      cargoArtifacts = craneLib.buildDepsOnly wasmArgs;
    }
  );

  # Kept apart from the cargo sources: editing the page rebuilds only the site.
  page = fs.toSource {
    root = root + "/demo";
    # `pkg/` holds a local build of the bindings when the tree is used as a path.
    fileset =
      fs.difference
      (fs.fileFilter (file: lib.any file.hasExt ["html" "js" "css"]) (root + "/demo"))
      (fs.maybeMissing (root + "/demo/pkg"));
  };

  versionJson = builtins.toJSON {
    inherit version;
    commit = rev;
  };
in
  pkgs.runCommand "livre-demo" {
    nativeBuildInputs = [pkgs.wasm-bindgen-cli];
    passAsFile = ["versionJson"];
    inherit versionJson;
  } ''
    mkdir -p $out/fixtures
    wasm-bindgen --target web --out-dir $out/pkg ${wasm}/lib/livre_wasm.wasm
    cp ${page}/* $out/
    for pdf in text.pdf letter.pdf text-xref-stream.pdf; do
      cp ${src}/crates/livre/tests/resources/$pdf $out/fixtures/
    done
    cp $versionJsonPath $out/version.json

    # Smoke check: a broken site fails the build, and so `nix flake check`.
    test -s $out/pkg/livre_wasm_bg.wasm
    test -s $out/index.html
    grep -q '"version"' $out/version.json
  ''
