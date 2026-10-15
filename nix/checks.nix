# The flake's checks, as crane derivations over one shared `cargoArtifacts`.
#
# Every check that inherits `cargoArtifacts` passes the same `cargoExtraArgs`:
# a check selecting other packages or features would recompile, inside itself,
# every dependency whose features change.
{
  pkgs,
  craneLib,
  craneLibMsrv,
}: let
  inherit (pkgs) lib;
  fs = lib.fileset;

  root = ../.;

  # The cargo sources, plus the PDFs the tests and examples read: an edit to
  # the README or the changelog rebuilds nothing.
  src = fs.toSource {
    inherit root;
    fileset = fs.unions [
      (craneLib.fileset.commonCargoSources root)
      (root + "/tests/resources")
    ];
  };

  commonArgs = {
    inherit src;
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

  msrv = craneLibMsrv.cargoBuild (
    msrvArgs
    // {
      cargoArtifacts = craneLibMsrv.buildDepsOnly msrvArgs;
    }
  );
}
