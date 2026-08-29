# Livre Features

Livre is a type-safe PDF parser written in Rust. It provides comprehensive tools for reading and extracting data from PDF documents.

## Index

- [features](features.md) — Core features and capabilities.

## Core Capabilities

### Type-Safe Parsing

Livre's primary feature is **type-safe extraction of PDF objects**. Every PDF primitive type has a dedicated Rust type:

- **Booleans**: `bool`
- **Integers**: `i32`, `i64`
- **Reals**: `f32`, `f64`
- **Strings**: `LiteralString`, `HexadecimalString`
- **Names**: `Name` (PDF keywords like `/Type`, `/Pages`)
- **Arrays**: `Vec<T>`
- **Dictionaries**: `RawDict` or custom structs via `FromRawDict`
- **Streams**: `Stream` with filter support
- **Null**: `Nil`
- **References**: `Reference<T>`, `OptRef<T>`

This ensures compile-time type safety when working with PDF data structures.

### PDF Structure Parsing

Livre can parse the complete PDF document structure:

- **Cross-reference tables**: Both plain and compressed (xref streams)
- **Trailer dictionaries**: Document metadata and xref location
- **Catalog**: Root object pointing to page tree
- **Page tree**: Hierarchical page organization with inheritance
- **Pages**: Individual page objects with content streams and resources

### Reference Resolution

PDF documents use indirect objects with references like `1 0 R`. Livre provides:

- **Lazy reference resolution**: Objects are only parsed when accessed
- **Multiple builder strategies**: `InMemoryBuilder` for in-memory documents
- **Type-safe references**: `Reference<T>` ensures type correctness

See [reference-resolution](reference-resolution.md) for details.

### Content Stream Parsing

Livre can parse PDF content streams to extract:

- **Text operators**: `Tj`, `TJ`, `Td`, `TD`, `Tm`, `T*`, etc.
- **Text state**: Font, font size, character spacing, word spacing
- **Text positioning**: Movement and transformation operators
- **Text delimiters**: Parentheses, brackets for text strings

See [content-extraction](content-extraction.md) for details.

### Stream Filter Support

PDF streams can be compressed. Livre currently supports:

- **FlateDecode**: Zlib compression (most common)

Planned support for:
- ASCII85Decode
- ASCIIHexDecode
- LZWDecode
- RunLengthDecode
- CCITTFaxDecode
- JBIG2Decode
- DCTDecode (JPEG)
- JPXDecode (JPEG2000)
- Crypt (encryption)

### Derive Macros

Livre provides proc-macro derive support for:

- **`#[derive(Extract)]`**: Generate parsing logic for custom types
- **`#[derive(FromRawDict)]`**: Generate dictionary extraction logic
- **`#[derive(BuildFromRawDict)]`**: Generate reference-resolving dictionary extraction

## Current Limitations

As of v0.5.0, Livre does **not** support:

- Font parsing (planned for v0.6.0)
- Image extraction
- Graphics state and path operators
- Encryption/decryption
- Most stream filters (only FlateDecode is implemented)
- PDF modification or creation

## Roadmap

See the README.md for the official roadmap:

- ✅ v0.1.0: Low-level extraction utilities for primitive PDF types
- ✅ v0.2.0: Parser for cross-reference dictionary
- ✅ v0.3.0: Eager, owned instance of the Builder trait
- ✅ v0.4.0: Parser for the general PDF structure (pages, catalog)
- ✅ v0.5.0: Parser for pages' content operators & text content
- 🔲 v0.6.0: Parser for fonts definition (simple fonts)
