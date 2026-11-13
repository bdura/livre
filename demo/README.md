# livre demo

A static page that runs livre, compiled to WebAssembly, on a PDF picked by the
visitor. pdf.js (loaded from jsdelivr at the version pinned in `app.ts`) renders
the current page under two overlays: livre's run markers and pdf.js's own text
items. An inspector beside the page shows the hovered or pinned run, the page's
text objects and livre's JSON for the page, and an amber strip names each stage
that failed. Nothing is uploaded anywhere.

No framework and no bundler. The scripts are TypeScript: `tsc` compiles each
`.ts` file into the `.js` beside it, which `index.html` loads as an ES module as
is. The wasm bindings' declarations (`pkg/livre_wasm.d.ts`, generated from the
Rust DTOs by tsify) type what livre returns, and `pdfjs.d.ts` types the slice
of pdf.js the page uses, by hand since pdf.js is loaded from a CDN. The
stylesheet is compiled too: the Tailwind CLI compiles `tailwind.css` into
`style.css`, scanning the page's `.html` and `.ts` files, so a class written in
a script must appear there as a literal string.

## Local development

Run `nix run .#demo` from the repository root, and again after each change:
it rebuilds whatever changed (the wasm, the scripts or the stylesheet) and
serves the site, as described below. A type error in the page fails the build.

## The built site

`nix build .#demo` assembles the deployable site (page, compiled stylesheet,
wasm bindings, fixtures and `version.json`) into `result/`.
`nix run .#demo` builds it and serves it with `static-web-server` on
<http://localhost:8000>; set `PORT` to use another port, and any extra arguments
are passed to `static-web-server`.
`nix flake check` builds it too, so a broken demo fails CI. The inspector's
footer shows the livre version and the commit it was built from, linked on
GitHub (suffixed `-dirty` for an uncommitted tree).

## Deployment

`.github/workflows/pages.yml` builds `.#demo` on every push to `main` and
deploys it to <https://bdura.github.io/livre/>. All asset paths in the page are
relative so that it works under that subpath. The repository's Pages source
must be set to "GitHub Actions" (Settings, Pages, Build and deployment): this is
a one-time manual step the workflow cannot do.
