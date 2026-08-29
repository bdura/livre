# Livre Architecture Decisions

This page documents the key architectural decisions made in Livre and their rationale.

## Index

- [architecture](architecture.md) — Design decisions and architecture overview.

## Design Philosophy

### Type Safety First

**Decision**: Every PDF primitive has a dedicated Rust type.

**Rationale**: PDF is a complex format with many data types. By providing strong typing:
- Compile-time errors catch many mistakes early
- IDE autocomplete works effectively
- Code is self-documenting
- Refactoring is safer

**Trade-off**: More boilerplate code for type definitions, but this is mitigated by derive macros.

### Layered Architecture

**Decision**: Separate concerns into distinct layers:
1. **Extraction layer** - Low-level byte parsing
2. **Structure layer** - PDF document structure
3. **Reference-following layer** - Indirect object resolution
4. **Content layer** - Page content parsing

**Rationale**: Each layer has a single responsibility:
- Extraction knows nothing about PDF structure
- Structure composes primitives without reference resolution
- Reference-following handles the object graph
- Content handles the highest-level parsing

**Trade-off**: Some complexity in navigating between layers, but this is outweighed by clarity and testability.

### Parse from the End

**Decision**: PDF parsing starts from the end of the file and works backwards.

**Rationale**: This follows the PDF specification's recommendation. The `startxref` marker at the end points to the cross-reference table, which is needed to locate all indirect objects. This approach:
- Is standard for PDF parsers
- Handles append-only PDF modifications correctly
- Builds the xref table before any object parsing

**Trade-off**: Cannot stream-parse PDFs (must load entire file), but this is acceptable for most use cases.

### Lazy Reference Resolution

**Decision**: References are resolved lazily, only when the object is actually accessed.

**Rationale**: PDF documents can have thousands of objects, but most use cases only need a subset. Lazy resolution:
- Reduces memory usage
- Improves performance for common use cases
- Allows partial parsing of damaged files

**Implementation**: The `Builder` trait provides `build_reference<T>()` which parses and returns the referenced object on demand.

### In-Memory Only (For Now)

**Decision**: `InMemoryDocument` loads the entire PDF into memory.

**Rationale**: Simplifies implementation significantly:
- No need for file I/O during parsing
- Random access is fast and easy
- Cross-reference table can be a simple HashMap

**Trade-off**: Cannot handle extremely large PDFs, but this is acceptable for the current use cases. A streaming/memory-mapped version could be added later.

### Use Winnow for Parsing

**Decision**: Use the `winnow` parser combinator library instead of hand-written parsers.

**Rationale**: Winnow provides:
- Excellent error reporting with context
- Composable parsers
- Good performance
- Active maintenance
- Familiar API for Rust developers

**Trade-off**: Slight overhead from the combinator approach, but this is negligible compared to I/O costs.

### Separate Derive Crate

**Decision**: Put proc-macros in a separate `livre-derive` crate.

**Rationale**: 
- Cleaner separation of concerns
- Faster compilation (proc-macros only recompiled when changed)
- Easier to maintain
- Standard Rust practice

**Trade-off**: Slightly more complex project structure, but this is minimal.

## Key Traits

### Extract

**Purpose**: Parse a type from a byte stream.

**Design**: Minimal trait with one method: `fn extract(input: &mut &BStr) -> ModalResult<Self>`

**Rationale**: Simple, composable, works with winnow combinators.

### Build

**Purpose**: Resolve references and construct complete objects.

**Design**: Works with a `Builder` to follow references: `fn build<B>(input: &mut &BStr, builder: &B) -> ModalResult<Self> where B: Builder`

**Rationale**: Separates parsing from reference resolution, enabling different builder strategies.

### Builder

**Purpose**: Provide a strategy for resolving references.

**Design**: Trait with `fn build_reference<T>(&self, Reference<T>) -> ModalResult<T>`

**Rationale**: Allows different implementations (in-memory, file-backed, etc.) without changing parsing logic.

## Why Not...?

### Why not use nom instead of winnow?

Winnow is a fork of nom with better error handling and active maintenance. The Livre author chose winnow for its superior error context features.

### Why not support PDF creation/modification?

Livre focuses on **reading** PDFs. Creation and modification would require:
- Different data structures (mutable vs immutable)
- Serialization logic
- Cross-reference table generation
- This would significantly complicate the codebase for a different use case.

### Why not use a DOM-like approach?

PDF's object graph is inherently different from HTML DOM:
- PDF has no parent pointers
- Objects are referenced by ID, not by tree position
- The graph can have cycles (though rare)
- A DOM approach would add unnecessary overhead.

## Architecture Evolution

### v0.1.0 - Primitive Extraction

Initial focus on parsing primitive PDF types. Established the `Extract` trait as the foundation.

### v0.2.0 - Cross-Reference Parsing

Added structure layer with xref table parsing. This enabled locating any object in the document.

### v0.3.0 - Builder Trait

Introduced the `Build` and `Builder` traits for reference resolution. This was a major architectural milestone that enabled useful work with PDF documents.

### v0.4.0 - Document Structure

Added parsing for catalog, page trees, and pages. This made Livre useful for most document inspection tasks.

### v0.5.0 - Content Parsing

Added content stream parsing with text operators. This enabled text extraction from PDF pages.

### v0.6.0 (Planned) - Font Parsing

Will add font dictionary parsing to enable accurate character positioning.
