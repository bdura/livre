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

4. **Content layer** (`content/`): Parsing of page content streams, including text
   operators and state management. This is the highest-level layer for extracting
   human-readable content.

The library deliberately does **not** render PDFs, edit PDFs, or handle encryption.
It focuses solely on reading and extracting data from existing PDF files in a
type-safe manner.

## Entry Point

Execution begins in `src/lib.rs` which exports the main `InMemoryDocument` type.
The primary entry point for users is:

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

```text
├── src/
│   ├── lib.rs                 # Main exports: InMemoryDocument, module re-exports
│   ├── document.rs            # InMemoryDocument, InMemoryBuilder - the main document types
│   ├── filtering.rs           # Stream filter implementations (FlateDecode, etc.)
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
│       ├── mod.rs             # Module exports and parse_text_object
│       ├── error.rs           # Content parsing errors
│       ├── operators/         # Content stream operators
│       │   ├── mod.rs         # Operator enum and parsing
│       │   └── text/          # Text-specific operators
│       │       ├── mod.rs     # Text operator categories
│       │       ├── delimiters.rs
│       │       ├── positioning.rs
│       │       ├── showing.rs # Text showing operators (Tj, TJ, etc.)
│       │       └── state.rs   # Text state operators
│       └── state/             # Content stream state management
│           ├── mod.rs
│           └── text.rs
│
├── livre-derive/              # Proc-macro crate for derive macros
│   ├── src/
│   │   ├── lib.rs            # Macro exports
│   │   ├── extract.rs        # Extract derive macro
│   │   ├── from_raw_dict.rs  # FromRawDict derive macro
│   │   ├── build_from_raw_dict.rs
│   │   └── utilities/        # Helper modules for macros
│   │       ├── mod.rs
│   │       ├── attr.rs
│   │       ├── extraction.rs
│   │       └── option.rs
│   └── Cargo.toml
│
├── examples/                  # Runnable examples
│   ├── content.rs            # Extract raw content streams
│   ├── doc.rs               # Parse document and extract page content
│   ├── text.rs              # Extract text operators from pages
│   └── text_objects.rs       # Work with text objects
│
└── tests/                    # Integration tests
    ├── content.rs           # Content parsing tests
    ├── text_objects.rs       # Text object tests
    └── resources/            # Test PDF files
```

### Key Components

**`InMemoryDocument`** (document.rs): The main document type. Holds the catalog
and a builder that can resolve references. Provides the `pages()` method to access
the page tree.

**`InMemoryBuilder`** (document.rs): Implements the `Builder` trait to resolve
references by looking up byte offsets in the cross-reference table and parsing
the referenced objects from the raw input.

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
streams and resources.

## Common tasks

- **Add support for a new PDF primitive type**: Implement `Extract` for your type
  in the `extraction/primitives/` or `extraction/special/` modules.
- **Add support for a new PDF structure type**: Define a struct and implement
  `Extract` and `Build` as needed, typically in the `structure/` module.
- **Add a new content operator**: Add a variant to `content::operators::Operator`
  and implement its parsing in the appropriate text or graphics submodule.
- **Add a new stream filter**: Implement the `Filtering` trait for your filter
  type in `filtering.rs`.
- **Create a custom builder**: Implement the `Builder` trait for your type to
  provide a custom reference resolution strategy.

## Cross-cutting concerns

**Error handling**: Uses `winnow`'s `ModalResult` type for parsing errors,
with `ContextError` providing contextual error messages. The `thiserror` crate
is used for custom error types like `ContentError`.

**Type safety**: Every PDF primitive has a dedicated Rust type. References are
strongly typed via the `Reference<T>` type, ensuring type safety when following
object graphs.

**Derive macros**: The `livre-derive` crate provides derive macros for `Extract`,
`FromRawDict`, and `BuildFromRawDict` to reduce boilerplate for custom types.

**Memory model**: `InMemoryDocument` loads the entire PDF into memory.
The `InMemoryBuilder` holds the raw bytes and cross-reference table, allowing
lazy resolution of objects without reparsing.

**Stream decoding**: The `Filtering` trait abstracts over different compression
algorithms. Currently only `FlateDecode` (zlib) is implemented; others return
unsupported errors.

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
