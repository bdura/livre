# Reference Resolution in Livre

How Livre handles PDF indirect objects and reference resolution.

## Index

- [reference-resolution](reference-resolution.md) — How Livre handles PDF indirect objects.

## The Problem

PDF documents use **indirect objects** to enable:
- Random access to objects
- Object reuse (multiple references to the same object)
- Append-only modifications

Each indirect object has a unique identifier: `N M` where:
- `N` = object number
- `M` = generation number (usually 0)

References to these objects use the syntax: `N M R`

Example:
```pdf
% Object definition
1 0 obj
/Type /Catalog
endobj

% Reference to that object
1 0 R
```

## Livre's Solution

Livre provides a **lazy reference resolution** system with three key traits:

### 1. Reference Type

`Reference<T>` represents a reference to an indirect object of type `T`:

```rust
pub struct Reference<T> {
    pub id: ReferenceId,
    pub _phantom: PhantomData<T>,
}
```

The generic parameter `T` ensures type safety - you can't accidentally use a reference to a `Page` where a reference to a `Font` is expected.

### 2. Build Trait

The `Build` trait defines how to resolve a type that may contain references:

```rust
pub trait Build {
    fn build<B>(input: &mut &BStr, builder: &B) -> ModalResult<Self>
    where
        B: Builder;
}
```

This is implemented for:
- Primitive types (they don't contain references, so just `Extract`)
- `Reference<T>` (follow the reference using the builder)
- `OptRef<T>` (either direct value or reference)
- Composite types (arrays, dictionaries, structs)

### 3. Builder Trait

The `Builder` trait defines how to follow a reference:

```rust
pub trait Builder {
    fn build_reference<T>(&self, Reference<T>) -> ModalResult<T>
    where
        T: Build;
}
```

This is the key abstraction that allows different reference resolution strategies.

## InMemoryBuilder

The primary implementation is `InMemoryBuilder`, which:

1. Holds the raw PDF bytes
2. Has a `HashMap<ReferenceId, RefLocation>` for the xref table
3. Can look up any object by its ID

When `build_reference` is called:

```rust
fn build_reference<T>(&self, Reference { id, .. }: Reference<T>) -> ModalResult<T> {
    // 1. Look up the object's location in the xref table
    let &offset = self.xrefs.get(&id)
        .ok_or(ErrMode::Backtrack(ContextError::new()))?;
    
    // 2. Get the bytes at that location
    let input = &mut self.input[offset..].as_ref();
    
    // 3. Parse the Indirect object
    let Indirect { id: reference_id, inner } = Indirect::parse(input, self.as_parser())?;
    
    // 4. Verify the ID matches
    debug_assert_eq!(reference_id, id);
    
    // 5. Return the inner value
    Ok(inner)
}
```

## Reference Locations

Livre supports two types of reference locations:

### Plain References

Most common format. The xref table contains:
```pdf
0000000100 00000 n 
```

This means: object 1, generation 0, is at byte offset 100, and is in-use (`n`).

Parsed as: `RefLocation::Plain(offset)`

### Compressed References (Object Streams)

PDF 1.5+ supports compressed cross-reference tables using object streams:

```pdf
1 0 obj
<<
  /Type /ObjStm
  /N 10
  /First 42
>>
stream
... compressed data ...
endstream
endobj
```

The xref table points to the stream object, and the index within it:

Parsed as: `RefLocation::Compressed { stream_id, index }`

When resolving:
1. Parse the object stream
2. Extract the specific object at the given index

## OptRef: Direct or Reference

Many PDF values can be either direct or indirect:

```pdf
% Direct
/FontSize 12

% Indirect (reference)
/FontSize 5 0 R
```

Livre provides `OptRef<T>` to handle this:

```rust
pub enum OptRef<T> {
    Direct(T),
    Ref(Reference<T>),
}
```

This is commonly used in dictionaries and arrays where values might be references.

## Building from Dictionaries

For structs extracted from dictionaries, Livre provides `BuildFromRawDict`:

```rust
pub trait BuildFromRawDict {
    fn build_from_raw_dict<B>(dict: &mut RawDict<'_>, builder: &B) -> ModalResult<Self>
    where
        B: Builder;
}
```

This is automatically derived for structs with the `#[derive(BuildFromRawDict)]` macro.

## Lazy Resolution

The key benefit of Livre's approach is **lazy resolution**:

```rust
// This doesn't parse the page yet
let page_ref: Reference<Page> = catalog.pages;

// This doesn't parse the page yet either
let opt_ref: OptRef<Page> = page_ref.into();

// NOW the page is parsed
let page: Page = builder.build_reference(page_ref)?;
```

This means:
- You can have many references without parsing everything
- Only accessed objects are parsed
- Memory usage is proportional to what you actually use
- Faster for partial document access

## Multiple Builder Strategies

While `InMemoryBuilder` is the primary implementation, the `Builder` trait allows other strategies:

### Potential Future Implementations

**FileBackedBuilder**:
- Keep PDF on disk
- Only load object bytes when needed
- Better for very large PDFs

**CachedBuilder**:
- Cache parsed objects
- Avoid re-parsing the same object multiple times
- Useful when the same object is referenced many times

**ValidatingBuilder**:
- Validate objects as they're parsed
- Check for corruption
- Verify cross-references

## Error Handling

Reference resolution can fail in several ways:

1. **Missing reference**: Object ID not found in xref table
   - Returns `ErrMode::Backtrack(ContextError::new())`

2. **Invalid offset**: Offset points outside the file
   - Returns `ErrMode::Backtrack(ContextError::new())`

3. **Type mismatch**: Object at location doesn't match expected type
   - Caught by type system (compile-time) or at runtime with downcasting

4. **Corrupted object**: Object bytes can't be parsed
   - Returns appropriate winnow error with context

## Best Practices

### Prefer References for Large Objects

```rust
// Good: Reference to page (lazy)
let page_ref: Reference<Page> = ...;

// Bad: Direct page (eager parsing)
let page: Page = builder.build_reference(page_ref)?;
// Only do this if you need the page immediately
```

### Use OptRef for Flexible Values

```rust
// Good: Accept either direct or reference
fn process_font_size(size: OptRef<f32>) {
    let size: f32 = match size {
        OptRef::Direct(v) => v,
        OptRef::Ref(r) => builder.build_reference(r)?,
    };
}
```

### Cache Frequently Accessed Objects

```rust
// If you access the same object many times
let catalog = builder.build_reference(doc.catalog)?;
// Now use catalog directly instead of re-resolving the reference
```

## Performance

Reference resolution is very fast:
- HashMap lookup: O(1)
- Byte slice: O(1) (just a pointer)
- Parsing: Depends on object complexity

For a typical PDF with 1000 objects:
- Building xref map: ~1ms
- Resolving a reference: ~1-10μs
- Parsing a simple object: ~1-5μs
- Parsing a complex object: ~10-100μs

The lazy approach means you only pay for what you use.
