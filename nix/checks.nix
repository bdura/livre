# The flake's checks, as crane derivations over one shared `cargoArtifacts`.
#
# Every check that inherits `cargoArtifacts` passes the same `cargoExtraArgs`:
# a check selecting other packages or features would recompile, inside itself,
# every dependency whose features change. `livre-wasm32` builds its own
# `cargoArtifacts`: dependencies compiled for the host are useless to another
# target.
{
  pkgs,
  fixtures,
  craneLib,
  craneLibMsrv,
  # The cargo sources, from `source.nix`.
  src,
}: let
  root = ../.;

  # The root manifest is a virtual workspace: the derivations take their name
  # from the main crate.
  commonArgs = {
    inherit src;
    inherit (craneLib.crateNameFromCargoToml {cargoToml = root + "/crates/livre/Cargo.toml";}) pname version;
    strictDeps = true;
    cargoExtraArgs = "--locked --workspace --all-features";
  };

  cargoArtifacts = craneLib.buildDepsOnly commonArgs;

  # The MSRV covers the libraries, not the dev-dependencies, so neither
  # derivation compiles test targets.
  msrvArgs =
    commonArgs
    // {
      doCheck = false;
    };

  # Only the library: the dev-dependencies need not compile for the browser.
  wasm32Args =
    commonArgs
    // {
      pname = "${commonArgs.pname}-wasm32";
      CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
      cargoExtraArgs = "--locked -p livre -p livre-wasm";
      doCheck = false;
    };
in {
  fmt = craneLib.cargoFmt {inherit src;};

  clippy = craneLib.cargoClippy (
    commonArgs
    // {
      inherit cargoArtifacts;
      cargoClippyExtraArgs = "--all-targets -- --deny warnings";
    }
  );

  test = craneLib.cargoTest (
    commonArgs
    // {
      inherit cargoArtifacts;
      cargoTestExtraArgs = "--all-targets -- --include-ignored";
    }
  );

  # `--all-targets` leaves doctests out: rust-lang/cargo#6669.
  doctest = craneLib.cargoDocTest (commonArgs // {inherit cargoArtifacts;});

  # The committed generated fixtures are what `nix/fixtures.nix` produces.
  fixtures = pkgs.runCommand "livre-fixtures-check" {} ''
    status=0
    for pdf in ${fixtures}/*.pdf; do
      name=$(basename "$pdf")
      if ! cmp "$pdf" "${src}/crates/livre/tests/resources/$name"; then
        echo "$name is out of date: run \`nix run .#fixtures\`" >&2
        status=1
      fi
    done
    [ "$status" -eq 0 ]
    touch $out
  '';

  msrv = craneLibMsrv.cargoBuild (
    msrvArgs
    // {
      cargoArtifacts = craneLibMsrv.buildDepsOnly msrvArgs;
    }
  );

  # livre and its wasm glue compile for the browser.
  livre-wasm32 = craneLib.mkCargoDerivation (
    wasm32Args
    // {
      cargoArtifacts = craneLib.buildDepsOnly (
        wasm32Args // {cargoCheckCommand = "cargo check --release";}
      );
      buildPhaseCargoCommand = "cargo check --release $cargoExtraArgs";
    }
  );
}
