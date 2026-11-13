# The Rust toolchains, from `rust-overlay`. Takes a `pkgs` with the overlay
# applied.
#
# `stable` is the dev shell's and the checks', so both name one derivation.
# `msrv` is the `rust-version` that `Cargo.toml` declares, so the `msrv` check
# tests the claim instead of restating it.
pkgs: let
  msrv = (pkgs.lib.importTOML ../Cargo.toml).package.rust-version;
in {
  stable = pkgs.rust-bin.stable.latest.default.override {
    extensions = [
      "rust-src"
      "rust-analyzer"
      "clippy"
      "rustfmt"
    ];
  };

  msrv = pkgs.rust-bin.stable.${msrv}.minimal;
}
