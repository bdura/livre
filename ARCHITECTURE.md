# Livre Architecture

Livre (pronounced [[livʁ]](https://en.wiktionary.org/wiki/File:Fr-un_livre-fr-ouest.ogg))
is a type-safe PDF parser written in Rust that optimises for **correct, type-safe
extraction of PDF document structure and content**. A reader who stops here should
know this is a library for parsing PDFs with strong type safety guarantees,
not a PDF renderer or generator.

## Glossary

- **PDF object** — A primitive data structure in PDF (boolean, integer, real,
  string, name, array, dictionary, stream, null).
- **Indirect object** — A PDF object with a unique identifier that can be
  referenced elsewhere in the document.
- **Reference** — A pointer to an indirect object, written as `N M R` where N is
  the object number and M is the generation number.
- **Cross-reference table (xref)** — A structure at the end of a PDF that maps
  indirect object identifiers to their byte offsets in the file.
- **Trailer** — A dictionary at the end of a PDF containing metadata including
  the root catalog reference and cross-reference table location.
- **Catalog** — The root object of a PDF document that points to the page tree
  and other document-level information.
- **Page tree** — A tree structure of page objects that defines the document's
  pages and their hierarchy.
- **Content stream** — A sequence of operators that define how to render content
  on a page (text, graphics, etc.).
- **Extract** — The trait defining how to parse a type from raw PDF bytes.
- **Build** — The trait defining how to resolve references and construct complete
  objects.
- **Builder** — A type that can follow references and resolve indirect objects
  into their concrete values.

## Bird's Eye View

Livre parses PDF documents by reading from the end of the file to locate
the cross-reference table, then following the chain of cross-reference
sections to build a complete mapping of object references to their locations.
Once the cross-reference table is assembled, it can resolve indirect references
to extract the document catalog, page tree, and page content.

The system is designed as a layered architecture:

1. **Extraction layer** (`extraction/`): Low-level parsing of primitive PDF
   types from byte streams. This layer knows nothing about PDF structure —
   it only understands how to read individual tokens and values.

2. **Structure layer** (`structure/`): Parsing of PDF document structure (catalog,
   pages, cross-reference tables, trails). This layer composes primitives to
   represent the document's skeleton.

3. **Reference-following layer** (`follow_refs/`): Mechanism to resolve indirect
   references and build complete objects. This is where the `Build` and `Builder`
   traits live, enabling lazy resolution of object graphs.

4. **Content layer** (`content/`): Parsing of page content streams into text runs.
   `TextObjects` walks a page's operators against its fonts, resolved beforehand
   by `Page::fonts`, and owns its graphics state: the CTM and the text state
   parameters (font, spacing, leading...), saved and restored together by `q`/`Q`,
   so a `Tf` outside `BT` or in an earlier text object still applies. Each text
   object it hands out yields `Run { start, end, characters, text }`, where
   `end` is `start` advanced past the shown glyphs, both in default user space,
   and each `Character` is one code with its own start, end and decoded text.
   Simple and `Type3` fonts decode codes through `/ToUnicode`, then their encoding
   and the Adobe Glyph List; a symbolic simple font without a predefined encoding
   starts from the built-in encoding of its embedded Type 1 program (`/FontFile`).
   Composite (`Type0`) fonts split strings by their encoding CMap's codespace
   ranges, map each code to a CID for the descendant CIDFont's widths, and decode
   through `/ToUnicode` only. A font's `WritingMode`,
   horizontal or vertical, owns the advance arithmetic of 9.4.4: `show` and `TJ`
   go through it, so a vertical font's glyphs advance down the page. Matrices
   and points are typed by coordinate space (`Glyph`, `Text`, `User`,
   `DefaultUser`). This is the highest-level layer for extracting
   human-readable content.

The library deliberately does **not** render PDFs, edit PDFs, or handle encryption.
It focuses solely on reading and extracting data from existing PDF files in a
type-safe manner.

## Entry Point

Execution begins in `crates/livre/src/lib.rs` which exports the main
`InMemoryDocument` type. The primary entry point for users is:

```rust
use livre::{extraction::Extract, InMemoryDocument};
use winnow::BStr;

let mut input: &BStr = &include_bytes!("document.pdf")[..];
let doc = InMemoryDocument::extract(&mut input)?;
```

The `InMemoryDocument::extract` function:

1. Finds the `startxref` marker at the end of the file
2. Locates and parses the cross-reference table and trailer
3. Follows any previous cross-reference table links to build the complete xref
   mapping
4. Constructs an `InMemoryBuilder` with the cross-reference data and raw input
5. Extracts the catalog (root) object
6. Returns an `InMemoryDocument` containing the catalog and builder

From there, users can call `doc.pages()` to get the page tree, then iterate over
pages and extract their content.

## Code Map

The repository is a virtual Cargo workspace: every crate lives under `crates/`,
and the root manifest only lists the members and their shared package metadata.

```text
├── Cargo.toml                 # Virtual workspace: members, shared package metadata
├── crates/
│   ├── livre/                 # The library, with its examples, tests and changelog
│   ├── livre-derive/          # Proc-macro crate for the derive macros
│   ├── livre-font-data/       # Glyph names and standard-font widths, as committed lookup tables
│   └── livre-wasm/            # Unpublished wasm glue behind the browser demo
├── demo/                      # Static page of the browser demo (TypeScript, no bundler)
└── nix/                       # Flake checks (crane), toolchains, generated fixtures, demo site
    ├── source.nix             # The cargo source fileset shared by checks and the demo
    ├── checks.nix             # Crane checks, including `livre-wasm32` (the crates on wasm32)
    ├── corpus.toml            # The benchmark's real-world PDFs: URL and hash of each
    ├── corpus.nix             # `packages.bench-corpus`: fetches them, adds a qpdf object-stream rewrite
    ├── demo.nix               # `packages.demo`: wasm build, wasm-bindgen, page, Tailwind CSS, fixtures
    │                          # (`nix run .#demo`, defined in flake.nix, serves it locally)
    ├── fixtures.nix           # Generated test fixtures and their drift check
    └── toolchain.nix          # rust-overlay toolchains, with the wasm32 target
```

### `crates/livre/`

```text
├── src/
│   ├── lib.rs                 # Main exports: InMemoryDocument, module re-exports
│   ├── document.rs            # InMemoryDocument, InMemoryBuilder - the main document types
│   ├── filtering.rs           # Stream filters (FlateDecode), each carrying its /DecodeParms
│   ├── filtering/
│   │   └── predictor.rs       # Predictor, validated from /DecodeParms; undoes PNG and TIFF after inflation
│   │
│   ├── extraction/            # Low-level primitive type extraction
│   │   ├── mod.rs             # Extract trait, FromRawDict trait, core types
│   │   ├── extract.rs         # Extract trait implementation
│   │   ├── from_raw_dict.rs   # FromRawDict trait for dictionary extraction
│   │   ├── primitives/        # Primitive PDF types: boolean, integer, real, string, etc.
│   │   └── special/           # Special PDF types: Name, Date, Stream, Reference, etc.
│   │       └── refs/          # Reference-related types
│   │       └── strings/       # String variants (Literal, Hexadecimal)
│   │
│   ├── follow_refs/           # Reference resolution and object building
│   │   ├── mod.rs             # Build trait, Builder trait, LivreBuilder
│   │   ├── build.rs           # Build trait for resolving references
│   │   ├── builder.rs         # Builder trait and implementations
│   │   └── from_raw_dict.rs   # BuildFromRawDict for building from dictionaries
│   │
│   ├── structure/             # PDF document structure parsing
│   │   ├── mod.rs             # Module exports and documentation
│   │   ├── catalog.rs         # Catalog (root) object
│   │   ├── content.rs         # Content stream types
│   │   ├── object_stream.rs   # Object stream (compressed objects)
│   │   ├── pages.rs           # Page tree and page objects
│   │   └── trailer_block/     # Trailer and cross-reference table parsing
│   │       ├── mod.rs         # XRefTrailerBlock, RefLocation
│   │       ├── block.rs       # Cross-reference table entries
│   │       ├── plain.rs       # Plain (non-compressed) reference locations
│   │       ├── ref_location.rs
│   │       ├── startxref.rs   # startxref marker parsing
│   │       ├── stream.rs      # Compressed cross-reference streams (xref streams)
│   │       └── trailer.rs     # Trailer dictionary
│   │
│   └── content/               # Page content parsing
│       ├── mod.rs             # Module exports
│       ├── error.rs           # Content parsing errors
│       ├── font/              # Font dictionaries, as far as positioning and decoding text needs
│       │   ├── mod.rs         # Font, dispatching on /Subtype, and Fonts
│       │   ├── code.rs        # Splitting a shown string into character codes, by codespace ranges
│       │   ├── widths.rs      # Glyph widths
│       │   ├── cid.rs         # CidFont: a Type0 descendant, /W, /DW, /W2, /DW2
│       │   ├── cmap/          # CMap<D>: codespace, bfchar, bfrange, cidchar, cidrange, lookup
│       │   │   ├── mod.rs     # CMap, CodespaceRange, lookup; WritingMode and its advance arithmetic
│       │   │   ├── cid.rs     # Cid, EncodingCMap: Identity-H/V or an embedded CMap, usecmap of an identity
│       │   │   ├── parse.rs   # PostScript tokens and the begin/end sections
│       │   │   └── unicode.rs # UnicodeText, ToUnicode = CMap<UnicodeText>
│       │   ├── encoding/      # Code to glyph name, for simple and Type3 fonts
│       │   │   ├── mod.rs     # Encoding, Flags, Base, the base table rule
│       │   │   ├── tables.rs  # The Annex D tables: Standard, WinAnsi, MacRoman, MacExpert, Symbol, ZapfDingbats
│       │   │   ├── glyph_name.rs # GlyphName and the AGL rules (uniXXXX, uXXXX, suffixes, ligatures)
│       │   │   ├── differences.rs # /Encoding as a name or a dictionary, /Differences
│       │   │   └── type1.rs   # Built-in encoding from a Type 1 program's clear text, before eexec
│       │   ├── decoder.rs     # Decoder: /ToUnicode first, then the encoding
│       │   ├── simple.rs      # Type1, MMType1, TrueType; standard 14 widths when /Widths is absent
│       │   ├── type3.rs       # Type3
│       │   └── composite.rs   # Type0: encoding CMap, descendant CIDFont, /ToUnicode
│       ├── operators/         # Content stream operators
│       │   ├── mod.rs         # Operator enum and parsing; OperatorName, an unmodelled operator's bytes
│       │   ├── graphics.rs    # Special graphics state operators (q, Q, cm)
│       │   └── text/          # Text-specific operators
│       │       ├── mod.rs     # Text operator categories
│       │       ├── delimiters.rs
│       │       ├── positioning.rs
│       │       ├── showing.rs # Text showing operators (Tj, TJ, etc.)
│       │       └── state.rs   # Text state operators
│       └── state/             # Content stream state management
│           ├── mod.rs
│           ├── matrix.rs      # Matrix<From, To>: transformation matrices between spaces
│           ├── space.rs       # Coordinate space markers, Point<Space> and Displacement<Space>
│           ├── graphics.rs    # GraphicsState: CTM, text state and their q/Q stack
│           └── text.rs        # TextObjects, TextObject, TextObjectStream, Run, text state
│
├── examples/                  # Runnable examples
│   ├── content.rs            # Extract raw content streams
│   ├── doc.rs               # Parse document and extract page content
│   ├── text.rs              # Extract text operators from pages
│   └── text_objects.rs       # Work with text objects
│
├── benches/                   # Extraction benchmark over the corpus of `LIVRE_BENCH_CORPUS`
│   ├── common/mod.rs          # The corpus, its expected-outcome table and check, the pipeline steps
│   ├── extraction.rs          # criterion: times open, fonts, content, text, end_to_end per document
│   └── allocations.rs         # Counting global allocator: allocations and bytes per document and step
│
└── tests/                    # Integration tests
    ├── content.rs           # Content parsing tests
    ├── text_objects.rs       # Text object tests, golden runs of the fixtures
    ├── xref_stream.rs       # Golden test: a predicted xref stream reads like the original
    ├── incremental_update.rs # Cross-reference sections chained through /Prev
    ├── derive.rs            # Behavioural tests for the derives, as a downstream crate
    ├── compile_fail.rs      # trybuild runner for tests/ui
    ├── ui/                  # Compile-fail cases with their expected .stderr
    └── resources/            # Test PDF files
```

Most fixtures in `tests/resources/` are hand-picked PDFs. Generated ones (today
`text-xref-stream.pdf`, `text.pdf` rewritten by qpdf with object streams and
a cross-reference stream using `/Predictor 12`) come from `nix/fixtures.nix`:
`nix run .#fixtures` regenerates them in place, and the `fixtures` flake check
fails when the committed files drift from what it produces.

### `crates/livre-derive/`

```text
├── src/
│   ├── lib.rs            # Entry points and crate docs (shapes, attributes)
│   ├── attrs.rs          # Syntactic parse of #[livre(...)], duplicates rejected
│   ├── plan.rs           # FieldPlan / ContainerPlan: modes, keys, validation
│   ├── dict.rs           # Generator for FromRawDict and BuildFromRawDict
│   ├── extract.rs        # Generator for Extract
│   └── generics.rs       # Trait bounds on type parameters
├── README.md
└── Cargo.toml
```

### `crates/livre-font-data/`

```text
├── src/
│   ├── table.rs          # The blob format: read in place, written by the tests
│   ├── glyph_list.rs     # Glyph name to Unicode text
│   ├── standard_font.rs  # StandardFont: alias resolution and widths
│   └── generate.rs       # cfg(test): parses the sources, checks the blobs are fresh
├── data/                 # Vendored AGL and AFM files, unmodified
├── tables/               # The .bin blobs built from data/ by the tests
└── LICENSE               # MIT, plus Adobe's notices
```

`livre` depends on it for glyph name lookup when decoding and for the standard
14 widths of fonts without `/Widths`. The vendored files are compiled by the
crate's own tests into `.bin` blobs under `tables/`, which are committed
and embedded with `include_bytes!`, so the crate needs no build script.
The tests fail when a committed blob no longer matches its source.

### `crates/livre-wasm/`

```text
├── src/
│   ├── lib.rs            # `analyze_pdf`, the wasm-bindgen export (wasm32 only)
│   ├── analyze.rs        # `analyze(&[u8]) -> Analysis`: runs livre stage by stage
│   └── dto.rs            # Serialisable mirror of what livre extracted, with TS interfaces (tsify)
└── tests/
    └── fixtures.rs       # Native tests of `analyze` on the livre fixtures
```

### `demo/`

```text
├── index.html            # The page: header controls, error strips, canvas and overlay svg, inspector
├── app.ts                # Loads the wasm and pdf.js, renders, navigates, reports errors, drives the inspector
├── overlay-livre.ts      # livre's layer: run boxes from start to end, a tick at each character's start, text object hulls on highlight
├── overlay-pdfjs.ts      # Ground truth: pdf.js text items as boxes
├── svg.ts                # The overlays' SVG element helper
├── pdfjs.d.ts            # Hand-written types for the slice of pdf.js the page uses
├── tsconfig.json         # Strict, emits each `.ts` as the `.js` beside it
├── tailwind.css          # Stylesheet source, compiled to `style.css` by the Tailwind CLI
└── README.md             # Local development and deployment
```

`demo/pkg/`, `demo/fixtures/`, `demo/style.css`, `demo/version.json` and
`demo/*.js` are generated and git-ignored.

### Key Components

Paths are relative to `crates/livre/src/`.

**`InMemoryDocument`** (document.rs): The main document type. Holds the catalog
and a builder that can resolve references. Provides the `pages()` method to access
the page tree.

**`InMemoryBuilder`** (document.rs): Implements the `Builder` trait to resolve
references by looking up byte offsets in the cross-reference table and parsing
the referenced objects from the raw input. An object compressed in an object
stream is read from that stream, decoded once and kept in the builder's
`ObjectStreamCache` for the other objects it holds.

**`Extract` trait** (extraction/mod.rs): Defines how to parse a type from a byte
stream. Implemented for all primitive PDF types and can be derived for custom types.

**`Build` trait** (follow_refs/build.rs): Defines how to resolve a type that may
contain references. Works with a `Builder` to follow references and construct
complete objects.

**`Builder` trait** (follow_refs/builder.rs): Defines how to follow a reference and
extract the referenced object. The `InMemoryBuilder` is the primary implementation.

**`Catalog`** (structure/catalog.rs): The root object of a PDF document,
containing references to the page tree and other document-level information.

**`Page`** (structure/pages.rs): Represents a page in the PDF, containing content
streams and resources. Its fonts are resolved eagerly, so the content layer needs
no document.

**`Font`** (content/font/mod.rs): A font dictionary, reduced to what positioning
and decoding text needs: splitting strings into codes, glyph widths,
and `Font::decode`, the text a code stands for. Simple and `Type3` fonts carry a
`Decoder`, which owns the precedence of ISO 32000-2 9.10.2: `/ToUnicode`, then the
glyph name their `Encoding` gives the code, through `livre-font-data`'s glyph list.
A symbolic simple font with no predefined `/Encoding` takes its base names from its
`/FontFile` program, which the descriptor keeps as an unresolved `FontFile` reference:
only such a font inflates its program, while every other font leaves it unread.
A composite font maps each code to a CID through its `EncodingCMap` and reads
its widths from the descendant `CidFont`; `Font::writing_mode` and
`Font::glyph_displacement` give the axis and size-1 displacement the advance uses.

**`TextObjects`** (content/state/text.rs): The entry point for text extraction.
It wraps a page's operator iterator and owns the graphics state for the whole
content stream: text state operators and `q`/`Q`/`cm` met outside `BT` update it,
and `next_object()` hands out a `TextObjectStream` borrowing it mutably, which yields
`Run`s. A dropped stream is skipped to its `ET`, text state operators still applied;
a missing `ET` is reported by the next `next_object()` as `IncompleteTextObject`.
Showing text with no font selected is `NoFontSelected`.

## Common tasks

- **Add support for a new PDF primitive type**: Implement `Extract` for your type
  in the `extraction/primitives/` or `extraction/special/` modules.
- **Add support for a new PDF structure type**: Define a struct and implement
  `Extract` and `Build` as needed, typically in the `structure/` module.
  Two crate-private macros, invoked beside the type, cover the trivial cases:
  `build_via_extract!` gives `Build` to a type holding no reference, and
  `name_enum!` gives `Extract` to a fieldless enum written as a PDF name.
- **Add a new content operator**: Add a variant to `content::operators::Operator`
  and implement its parsing in the appropriate text or graphics submodule.
  Until then, the operator parses as `Operator::NotImplemented`, which holds its
  bytes inline in an `OperatorName` (an operator token is one to three bytes).
  A text operator also implements `TextStateOperation`, `TextPositioningOperation`
  or `TextShowingOperation`, depending on whether it mutates the text state
  parameters, the text matrix or the text object's buffered strings.
- **Add a new font subtype**: Add a variant to `content::font::Font`, backed by
  its own module in `content/font/`, and handle it in each of `Font`'s matches.
  The text advance only goes through `Font`, so it needs no change.
- **Add a predefined encoding**: Add its 256-entry glyph name table to
  `content/font/encoding/tables.rs`, extend the table test there that resolves
  every name, and map its `/Encoding` name (or the `/BaseFont` that implies it)
  in the base table rule of `content/font/encoding/mod.rs`.
- **Add a predefined CMap**: Vendor its file under `crates/livre-font-data/data/`
  and compile it to a blob there, as the glyph list and AFM files are (see the
  crate's README), then map its name in `EncodingCMap`'s predefined-name match in
  `content/font/cmap/cid.rs`, which today knows only `Identity-H` and `Identity-V`
  and warns on every other name.
- **Add a new stream filter**: In `filtering.rs`, add a variant to `Filter`,
  backed by a type implementing `Filtering::decode(&self, bytes)`. The filter is
  extracted from its `/Filter` name; if it takes parameters, its arm of
  `Filter::with_params` validates its `/DecodeParms` entry and stores it in the
  filter, where `decode` reads it.
- **Create a custom builder**: Implement the `Builder` trait for your type to
  provide a custom reference resolution strategy.
- **Update the font data**: Replace the file under `crates/livre-font-data/data/`,
  then follow the crate's README to rebuild the blobs.
- **Run the benchmark**: Inside the dev shell (`nix develop`), which sets
  `LIVRE_BENCH_CORPUS`, run `cargo bench -p livre --bench extraction` for the
  timings (filter steps with a trailing `-- text`) and
  `cargo bench -p livre --bench allocations` for the allocation table, which
  is deterministic and compares between runs with `diff`. To compare two states
  of the code, run `cargo bench -p livre --bench extraction -- --save-baseline before`
  on the first, then `cargo bench -p livre --bench extraction -- --baseline before`
  on the second. Trust only the `fonts`, `text` and `end_to_end` changes: the
  sub-millisecond `open` and `content` benches settle on a different level in
  each run, up to 13% apart on unchanged code.
- **Add a document to the benchmark**: Add a `[[document]]` entry with
  `bench = true` to `nix/corpus.toml` (a URL that names a fixed version, and its
  sha256), then a row for it in `EXPECTED` in `crates/livre/benches/common/mod.rs`
  with its pages, failing pages and characters. Both bench targets check every
  document against its row before measuring anything.

## Cross-cutting concerns

**Error handling**: Uses `winnow`'s `ModalResult` type for parsing errors,
with `ContextError` providing contextual error messages. The `thiserror` crate
is used for custom error types like `ContentError`.

**Type safety**: Every PDF primitive has a dedicated Rust type. References are
strongly typed via the `Reference<T>` type, ensuring type safety when following
object graphs.

**Derive macros**: The `livre-derive` crate provides derive macros for `Extract`
(tuple structs, unit structs, enums of single-field variants), `FromRawDict`
and `BuildFromRawDict` (structs with named fields). Each dictionary field reads
the `PascalCase` form of its name as key, tunable with `#[livre(...)]` attributes:
`rename = "Key"`, `from = Type` (extract, then `Into`), `default` (bare,
literal, closure or expression; applied after `from`) and `flatten`.
Generated code uses absolute `::livre::` and `::winnow::` paths; `livre` itself
declares `extern crate self as livre` so those paths resolve inside the crate too.
Misuse is a spanned compile error, checked by the trybuild cases in `tests/ui`.
An `Option<T>` field builds through `Built<T>`, as do the items of `Vec`, arrays and
`MaybeArray`, so a reference there is followed; a required field is read as written.
A type that holds a reference itself sets `Build::FOLLOW` to `false` and is never
followed: `Reference<T>`, `OptRef<T>` and `Object`. A field keeps a reference
unresolved by being typed as one, as `FontDescriptor::font_file` and
`ObjectStreamDict::extends` are.

**Memory model**: `InMemoryDocument` loads the entire PDF into memory.
The `InMemoryBuilder` holds the raw bytes and cross-reference table, allowing
lazy resolution of objects without reparsing. It also caches decoded object
streams by stream number, so a stream holding many objects is inflated and its
header parsed once per builder rather than once per object. The cache is a
`Mutex<HashMap<usize, Arc<ObjectStream>>>`, which keeps the builder `Send` and
`Sync`. No lock is held while a stream is built, since building re-enters
`build_reference`: two threads may each build the same stream, and the first
one stored is kept. Failed builds are not cached, and nothing is evicted. The
cache is derived from the input rather than part of the builder's value: a
clone starts with an empty cache, and two builders compare equal whatever their
caches hold.

**Stream decoding**: The `Filtering` trait abstracts over different compression
algorithms. Currently only `FlateDecode` (zlib) is implemented; others return
unsupported errors. When the stream dictionary is read, `StreamConfig` pairs
each `/Filter` entry with its `/DecodeParms` entry (a single dictionary or an
array parallel to it, `null` for no parameters) and validates the parameters,
so invalid ones fail there, before the content is read. A `/DecodeParms` of
another length than `/Filter` is warned and tolerated. FlateDecode carries a
`Predictor` and undoes PNG (10 to 15) and TIFF (2) predictors after inflating.

**Browser demo**: `crates/livre-wasm` (unpublished) exposes `analyze_pdf`,
which runs livre on a PDF's bytes and returns plain DTOs (`dto.rs`), so serde
stays out of livre. The DTOs derive `tsify::Tsify`: the page's TypeScript is
type-checked against the generated declarations, so a DTO change that breaks
it fails the build. `nix/demo.nix` assembles the site (wasm, `tsc`, Tailwind,
fixtures), which is a flake check, deployed to GitHub Pages and served locally
by `nix run .#demo`. The page overlays livre's runs on pdf.js's text layer,
the visual harness for the font work.

**Benchmark**: Two bench targets in `crates/livre/benches/` measure text
extraction in steps (open, fonts, content, text, and end to end): `extraction`
times them with criterion, `allocations` counts the allocations and bytes each
step allocates. The documents are real-world PDFs that Nix fetches by hash from
`nix/corpus.toml` (`nix/corpus.nix`), plus two fixtures of `tests/resources/`;
none of the fetched ones is in the repository. The flake's test check runs each
target once, which checks every document's expected outcome, but no timing or
count is asserted: the benchmark informs optimisation work, it is not a
regression guard. The expected character counts include U+FFFD, one per code
without text, so they do not detect a decoding regression.

**Invariants**:

- References must always resolve to valid objects (enforced by `Builder`
  implementations returning errors for invalid references)
- Cross-reference tables must form a valid linked list (enforced by the parsing
  logic in `document.rs`)
- Page trees must be valid (enforced at parse time by the page tree parsing logic)

**Conventions**:

- Module documentation (`mod.rs` files) contain detailed explanations of the
  module's purpose and design
- Type names match PDF specification terminology where possible
- Parsing is always done from byte streams using `winnow` combinators
- References are resolved lazily — only when the referenced object is actually
  needed
