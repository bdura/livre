# The cargo sources the flake's derivations build from, plus the PDFs the tests
# and examples read and the trybuild `.stderr` snapshots: an edit to the README,
# the changelog or the demo page rebuilds no cargo derivation.
{
  lib,
  craneLib,
}: let
  fs = lib.fileset;
  root = ../.;
in
  fs.toSource {
    inherit root;
    fileset = fs.unions [
      (craneLib.fileset.commonCargoSources root)
      (root + "/crates/livre/tests/resources")
      (root + "/crates/livre/tests/ui")
    ];
  }
