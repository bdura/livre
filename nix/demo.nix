# The browser demo as a static site: the page from `demo/` with its stylesheet
# compiled by the Tailwind CLI, livre compiled to wasm32 with its wasm-bindgen
# glue in `pkg/`, the test PDFs in `fixtures/`, and a `version.json` the
# inspector footer reads.
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
    # `pkg/` and `style.css` hold local builds when the tree is used as a path.
    fileset =
      fs.difference
      (fs.fileFilter (file: lib.any file.hasExt ["html" "js" "css"]) (root + "/demo"))
      (fs.unions [
        (fs.maybeMissing (root + "/demo/pkg"))
        (fs.maybeMissing (root + "/demo/style.css"))
      ]);
  };

  versionJson = builtins.toJSON {
    inherit version;
    commit = rev;
  };
in
  pkgs.runCommand "livre-demo" {
    nativeBuildInputs = [pkgs.wasm-bindgen-cli pkgs.tailwindcss_4];
    passAsFile = ["versionJson"];
    inherit versionJson;
  } ''
    mkdir -p $out/fixtures
    wasm-bindgen --target web --out-dir $out/pkg ${wasm}/lib/livre_wasm.wasm
    cp ${page}/*.html ${page}/*.js $out/
    # Scans the page's markup and scripts, per the `@source` lines of `tailwind.css`.
    tailwindcss --minify -i ${page}/tailwind.css -o $out/style.css
    for pdf in text.pdf letter.pdf text-xref-stream.pdf; do
      cp ${src}/crates/livre/tests/resources/$pdf $out/fixtures/
    done
    cp $versionJsonPath $out/version.json

    # Smoke check: a broken site fails the build, and so `nix flake check`.
    test -s $out/pkg/livre_wasm_bg.wasm
    test -s $out/index.html
    grep -q 'layer-chip' $out/style.css
    grep -q '"version"' $out/version.json
  ''
