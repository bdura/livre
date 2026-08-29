# Livre Parsing Approach

Details on how Livre parses PDF documents.

## PDF Parsing Strategy

Livre follows the PDF specification's recommendation to parse from the end of the
file:

1. **Find `startxref`** - Locate the byte offset to the last cross-reference table
2. **Parse xref table** - Read the cross-reference table and trailer
3. **Follow xref chain** - If there's a previous xref table, follow it
4. **Build xref map** - Create a complete mapping of object IDs to byte offsets
5. **Parse catalog** - Extract the root catalog object using the xref map
6. **Return document** - Provide access to the catalog and xref map via
   `InMemoryDocument`

## Parsing Layers

### Extraction Layer (Low-Level)

Located in `src/extraction/`, this layer handles:

- **Primitive types**: integers, reals, booleans, strings, names, null
- **Composite types**: arrays, dictionaries, streams
- **Special types**: references, indirect objects, dates, rectangles

The `Extract` trait is the core:

```rust
pub trait Extract<'de> {
    fn extract(input: &mut &'de BStr) -> ModalResult<Self>
    where
        Self: Sized;
}
```

All parsing is done using `winnow` combinators, which provide:

- Composable parsers
- Excellent error context
- Backtracking support

### Structure Layer

Located in `src/structure/`, this layer composes primitives to represent PDF
document structure:

- **Trailer block**: `StartXRef`, `XRefTrailerBlock`, `Trailer`, `RefLocation`
- **Cross-reference**: `block.rs` for xref entries, `stream.rs` for compressed xrefs
- **Catalog**: Root object with page tree reference
- **Pages**: Page tree nodes and individual pages
- **Content**: Content streams

This layer does **not** resolve references - it only parses the structure.

### Reference-Following Layer

Located in `src/follow_refs/`, this layer provides the mechanism to resolve
indirect references:

- **`Build` trait**: Defines how to resolve a type that may contain references
- **`Builder` trait**: Defines how to follow a reference and extract the object
- **`InMemoryBuilder`**: Primary implementation that uses the xref map

Example:

```rust
// Reference<T> is extracted from the byte stream
let reference: Reference<Page> = extract(input)?;

// Build resolves it using a Builder
let page: Page = builder.build_reference(reference)?;
```

### Content Layer

Located in `src/content/`, this layer handles page content stream parsing:

- **Operators**: Text operators (`Tj`, `TJ`, `Td`, etc.)
- **State**: Text state management (font, spacing, matrix)
- **Text objects**: Grouped text operations

Content parsing uses the same `Extract` trait but works with content-specific types.

## Key Parsing Patterns

### Indirect Objects

PDF indirect objects have the format:

```pdf
1 0 obj
(Hello World)
endobj
```

Parsed as:

```rust
Indirect {
    id: ReferenceId { num: 1, gen: 0 },
    inner: PDFString("Hello World".into()),
}
```

### References

PDF references have the format: `1 0 R`

Parsed as:

```rust
Reference {
    id: ReferenceId { num: 1, gen: 0 },
    _phantom: PhantomData<PDFString>,
}
```

The `Reference<T>` type is generic, allowing type-safe reference resolution.

### Dictionaries

PDF dictionaries have the format:

```pdf
<<
  /Type /Catalog
  /Pages 2 0 R
>>
```

Parsed as `RawDict` or custom structs via `FromRawDict`:

```rust
Catalog {
    type_: Name("Catalog"),
    pages: Reference<PageTreeNode>(ReferenceId { num: 2, gen: 0 }),
}
```

### Arrays

PDF arrays have the format: `[1 2 3]`

Parsed as `Vec<T>` where `T: Extract`:

```rust
Vec::<i32>::extract(input)?; // [1, 2, 3]
```

### Streams

PDF streams have the format:

```pdf
1 0 obj
<<
  /Length 13
  /Filter /FlateDecode
>>
stream
Hello, World!
endstream
endobj
```

Parsed as:

```rust
Stream {
    dict: RawDict,
    data: Vec<u8>, // Decoded using the Filter
}
```

## Cross-Reference Table Parsing

The xref table is the most critical part of PDF parsing. Livre handles:

### Plain XRef Tables

Format:

```pdf
xref
0 5
0000000000 65535 f 
0000000010 00000 n 
0000000100 00000 n 
...
```

Parsed into `RefLocation::Plain(offset)` entries.

### Compressed XRef Streams

Format: A stream object containing compressed xref data.

Parsed into `RefLocation::Compressed { stream_id, index }` entries.

### XRef Chain

PDFs can have multiple xref tables (for incremental updates). Livre follows the
chain:

```pdf
trailer
<<
  /Size 10
  /Prev 1234  % offset to previous xref
>>
```

The parser follows `Prev` offsets until all xref tables are collected.

## Content Stream Parsing

Content streams contain operators that define how to render content:

```pdf
BT
  /F1 12 Tf
  100 200 Td
  (Hello) Tj
ET
```

Parsing approach:

1. **Tokenize**: Split into operators and operands
2. **Classify**: Identify operator type (text, graphics, etc.)
3. **Group**: Organize into logical operations

Livre provides:

- `Operator` enum for all supported operators
- `TextOperator` for text-specific operators
- `parse_text_object` for parsing complete text objects

## Error Handling

Livre uses winnow's error system:

- **`ContextError`**: Provides context about what was expected
- **`ErrMode::Backtrack`**: Parsing failed but can retry
- **`ErrMode::Cut`**: Parsing failed at a cut point (no backtracking)
- **`ErrMode::Assert`**: Internal assertion failed

Context is automatically added for:

- Expected tokens
- Expected types
- Expected structures

Example error:

```text
Error: expected integer, found 'a' at byte offset 1234
```

## Performance Considerations

### Lazy Parsing

- Objects are only parsed when accessed
- Cross-reference table is built upfront
- Raw bytes are kept in memory for on-demand parsing

### Memory Usage

- Entire PDF loaded into memory
- Xref map: O(n) where n = number of objects
- Parsed objects: O(m) where m = number of accessed objects

### Parsing Speed

- Primitive parsing: Very fast (simple token matching)
- Reference resolution: O(1) lookup + parsing cost
- Content parsing: Depends on content complexity

## Testing

Livre uses:

- **Unit tests**: For individual parsing functions
- **Integration tests**: For complete document parsing
- **Test PDFs**: Real PDF files in `tests/resources/`

Test approach:

```rust
#[test]
fn test_parse_document() {
    let mut input: &BStr = &include_bytes!("test.pdf")[..];
    let doc = InMemoryDocument::extract(&mut input).unwrap();
    assert_eq!(doc.pages().unwrap().len(), 1);
}
```
