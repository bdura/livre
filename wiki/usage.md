# Livre Usage Guide

How to use Livre to parse PDF documents and extract their content.

## Quick Start

```rust
use livre::{extraction::Extract, InMemoryDocument};
use winnow::BStr;

// Load PDF bytes
let pdf_bytes = std::fs::read("document.pdf").unwrap();
let mut input: &BStr = pdf_bytes.as_slice().as_ref();

// Parse the document
let doc = InMemoryDocument::extract(&mut input).unwrap();

// Get all pages
let pages = doc.pages().unwrap();
println!("Found {} pages", pages.len());

// Extract content from each page
for page in pages {
    let content = page.build_content(&doc).unwrap();
    // content is a Vec<u8> of the page's content stream
}
```

## Basic Usage

### Loading a Document

```rust
use livre::{extraction::Extract, InMemoryDocument};
use winnow::BStr;

let mut input: &BStr = &include_bytes!("document.pdf")[..];
let doc = InMemoryDocument::extract(&mut input)?;
```

The `InMemoryDocument` contains:
- `catalog`: The root `Catalog` object
- `builder`: An `InMemoryBuilder` for resolving references

### Accessing Pages

```rust
let pages = doc.pages()?;
for page in pages {
    // page: Page struct
    // page.parent: Option<Reference<PageTreeNode>>
    // page.contents: Vec<Reference<ContentStream>>
    // page.resources: Resources
    // page.media_box: Option<Rectangle>
}
```

### Building Content

To get the raw content stream bytes:

```rust
for page in doc.pages()? {
    let content_bytes = page.build_content(&doc)?;
    // content_bytes: Vec<u8>
}
```

### Parsing Content Operators

```rust
use livre::content::operators::Operator;
use winnow::{ascii::multispace0, combinator::{iterator, preceded}, BStr};

let content = page.build_content(&doc)?;
let mut stream = BStr::new(&content);

let mut it = iterator(&mut stream, preceded(multispace0, Operator::extract));

for operator in &mut it {
    match operator {
        Operator::BeginText(_) => println!("Begin text"),
        Operator::Text(text_op) => println!("Text: {:?}", text_op),
        // ... other operators
    }
}

it.finish()?;
```

## Common Patterns

### Extracting All Text

```rust
use livre::content::operators::{text::TextOperator, Operator};

for page in doc.pages()? {
    let content = page.build_content(&doc)?;
    let mut stream = BStr::new(&content);
    
    let mut it = iterator(&mut stream, preceded(multispace0, Operator::extract));
    
    for operator in &mut it {
        if let Operator::Text(TextOperator::TextShowingOperator(text)) = operator {
            print!("{}", text);
        }
    }
    
    it.finish()?;
}
```

### Following References Manually

```rust
use livre::extraction::Reference;

// If you have a Reference<T> from parsing
let reference: Reference<SomeType> = ...;
let resolved = doc.builder.build_reference(reference)?;
// resolved: SomeType
```

### Working with Dictionaries

```rust
use livre::extraction::{FromRawDict, RawDict};
use winnow::BStr;

// If you have a raw dictionary
let mut dict: RawDict = ...;

// Extract a known type
let catalog: Catalog = FromRawDict::from_raw_dict(&mut dict)?;

// Or extract a custom struct
#[derive(FromRawDict)]
struct MyDict {
    #[livre(name = "Type")]
    type_: Name,
    #[livre(name = "Count")]
    count: i32,
}

let my_dict = MyDict::from_raw_dict(&mut dict)?;
```

## Error Handling

Livre uses `winnow`'s `ModalResult` type, which is a `Result<T, ErrMode<E>>`.

```rust
use winnow::error::ErrMode;

match InMemoryDocument::extract(&mut input) {
    Ok(doc) => { /* success */ }
    Err(ErrMode::Backtrack(err)) => {
        // Parsing failed but we can backtrack
        eprintln!("Parse error: {:?}", err);
    }
    Err(ErrMode::Cut(err)) => {
        // Parsing failed at a cut point (no backtracking possible)
        eprintln!("Fatal parse error: {:?}", err);
    }
    Err(ErrMode::Assert(err)) => {
        // Internal assertion failed
        eprintln!("Assertion error: {:?}", err);
    }
}
```

## Custom Types

### Implementing Extract

```rust
use livre::extraction::{Extract, extract};
use winnow::{combinator::trace, BStr, ModalResult, Parser};

struct MyType {
    a: i32,
    b: String,
}

impl Extract<'_> for MyType {
    fn extract(input: &mut &BStr) -> ModalResult<Self> {
        let a: i32 = extract(input)?;
        let b: String = extract(input)?;
        Ok(Self { a, b })
    }
}
```

### Using Derive Macros

```rust
use livre_derive::Extract;

#[derive(Extract)]
struct MyTupleType(i32, String);

#[derive(FromRawDict)]
struct MyDictType {
    #[livre(name = "A")]
    a: i32,
    #[livre(name = "B")]
    b: String,
}
```

## Tips and Tricks

### Debugging Parsing Issues

Use `trace` from winnow to add debug output:

```rust
use winnow::combinator::trace;

let parser = trace("my-parser", |i: &mut &BStr| {
    // your parsing logic
});
```

### Handling Compressed Objects

Livre automatically handles object streams (compressed cross-reference tables). You don't need to do anything special - the `InMemoryBuilder` handles it transparently.

### Memory Considerations

`InMemoryDocument` loads the entire PDF into memory. For very large PDFs:
- Consider processing pages one at a time
- Don't hold references to all pages simultaneously
- Use streaming parsers for content when possible

## Examples

See the `examples/` directory for runnable examples:
- `content.rs` - Extract raw content streams
- `doc.rs` - Parse document and extract page content
- `text.rs` - Extract text operators from pages
- `text_objects.rs` - Work with text objects
