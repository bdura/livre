{
  description = "Livre's reproducible development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
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

        fixtures = import ./nix/fixtures.nix pkgs;
      in {
        # `nix flake check` runs these; so does CI.
        checks = import ./nix/checks.nix {
          inherit pkgs fixtures;
          craneLib = (crane.mkLib pkgs).overrideToolchain (_: toolchain.stable);
          craneLibMsrv = (crane.mkLib pkgs).overrideToolchain (_: toolchain.msrv);
        };

        # Writes the generated fixtures into the tree; run from the repository root.
        apps.fixtures = {
          type = "app";
          program = pkgs.lib.getExe (pkgs.writeShellApplication {
            name = "fixtures";
            text = ''
              dest=crates/livre/tests/resources
              if [ ! -d "$dest" ]; then
                echo "fixtures: $dest not found; run from the repository root" >&2
                exit 1
              fi
              for pdf in ${fixtures}/*.pdf; do
                name=$(basename "$pdf")
                cp "$pdf" "$dest/$name"
                # Copies of store paths are read-only.
                chmod u+w "$dest/$name"
              done
            '';
          });
        };

        devShells.default =pkgs.mkShell {
          packages = [
            toolchain.stable
            pkgs.prek
            # The browser demo's crate pins `wasm-bindgen` to this version.
            pkgs.wasm-bindgen-cli
          ];

          RUST_BACKTRACE = "1";
        };
      }
    );
}
