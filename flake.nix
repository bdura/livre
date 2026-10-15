{
  description = "Livre's reproducible development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    # The checks; crane takes no inputs of its own.
    crane.url = "github:ipetkov/crane";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    rust-overlay,
    crane,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [(import rust-overlay)];
        };

        toolchain = import ./nix/toolchain.nix pkgs;
      in {
        # `nix flake check` runs these; so does CI.
        checks = import ./nix/checks.nix {
          inherit pkgs;
          craneLib = (crane.mkLib pkgs).overrideToolchain (_: toolchain.stable);
          craneLibMsrv = (crane.mkLib pkgs).overrideToolchain (_: toolchain.msrv);
        };

        devShells.default = pkgs.mkShell {
          packages = [
            toolchain.stable
            pkgs.prek
          ];

          RUST_BACKTRACE = "1";
        };
      }
    );
}
