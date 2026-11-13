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

        craneLib = (crane.mkLib pkgs).overrideToolchain (_: toolchain.stable);

        src = import ./nix/source.nix {
          inherit (pkgs) lib;
          inherit craneLib;
        };
      in {
        # `nix flake check` runs these; so does CI.
        checks =
          import ./nix/checks.nix {
            inherit pkgs fixtures craneLib src;
            craneLibMsrv = (crane.mkLib pkgs).overrideToolchain (_: toolchain.msrv);
          }
          // {
            inherit (self.packages.${system}) demo;
            demo-test = import ./nix/demo-test.nix {
              inherit pkgs;
              inherit (self.packages.${system}) demo;
            };
          };

        # The browser demo, deployed to GitHub Pages.
        packages.demo = import ./nix/demo.nix {
          inherit pkgs craneLib src;
          rev = self.rev or self.dirtyRev or "unknown";
        };

        # Serves the built demo locally; extra arguments go to static-web-server.
        apps.demo = {
          type = "app";
          program = pkgs.lib.getExe (pkgs.writeShellApplication {
            name = "demo";
            runtimeInputs = [pkgs.static-web-server];
            text = ''
              port=''${PORT:-8000}
              echo "Serving the livre demo on http://localhost:$port"
              # Without `--cache-control-headers false`, the browser keeps `app.js` and the
              # wasm bindings for up to a year: their names do not change between builds.
              exec static-web-server --root ${self.packages.${system}.demo} --host 127.0.0.1 --port "$port" \
                --cache-control-headers false "$@"
            '';
          });
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
            # Compiles the demo page's `tailwind.css` into `style.css`.
            pkgs.tailwindcss_4
            # Compiles the demo page's `.ts` files, run as `tsc -p demo`.
            pkgs.typescript
          ];

          RUST_BACKTRACE = "1";
        };
      }
    );
}
