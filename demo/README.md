# livre demo

A static page that runs livre, compiled to WebAssembly, on a PDF picked by the
visitor. pdf.js (loaded from jsdelivr at the version pinned in `app.js`) renders
the current page; livre's analysis of the whole document is shown as JSON, with
a banner for each stage that failed. Nothing is uploaded anywhere.

No framework and no build step for the page itself: `index.html`, `style.css`
and the `app.js` ES module are served as is.

## Local development

From the repository root, inside `nix develop` (which provides the wasm32
target and a `wasm-bindgen` matching the crate's pinned version):

```sh
cargo build -p livre-wasm --target wasm32-unknown-unknown --release
wasm-bindgen --target web --out-dir demo/pkg \
  target/wasm32-unknown-unknown/release/livre_wasm.wasm
mkdir -p demo/fixtures && cp crates/livre/tests/resources/*.pdf demo/fixtures/
python3 -m http.server -d demo 8000
```

then open <http://localhost:8000>. Re-run the first two commands after changing
livre or `crates/livre-wasm`; page changes only need a reload.

`demo/pkg/`, `demo/fixtures/` and `demo/version.json` are generated and
git-ignored. Without `version.json` the footer simply omits the version.

## The built site

`nix build .#demo` assembles the deployable site (page, wasm bindings,
fixtures and `version.json`) into `result/`.
`nix run .#demo` builds it and serves it with `static-web-server` on
<http://localhost:8000>; set `PORT` to use another port, and any extra arguments
are passed to `static-web-server`.
`nix flake check` builds it too, so a broken demo fails CI. Its footer shows
the livre version and the commit it was built from (suffixed `-dirty` for an
uncommitted tree).

## Deployment

`.github/workflows/pages.yml` builds `.#demo` on every push to `main` and
deploys it to <https://bdura.github.io/livre/>. All asset paths in the page are
relative so that it works under that subpath. The repository's Pages source
must be set to "GitHub Actions" (Settings, Pages, Build and deployment): this is
a one-time manual step the workflow cannot do.
