# Design decisions and rationale

## Object parsing and re-use

### The problem

Since PDF objects may be re-used, there's a chance we might re-parse some multiple times.

This effect is compounded by the presence of object streams,
which should not be parsed again every time one of its objects is extracted.

### Potential solutions

In the case of object streams, memoization? Parsing once and for all at the beginning of the process.

Pros:

- should simplify processing: we go through the xrefs to map every reference that's needed, then parse them all at once.

Cons:

- we may lose some type information that's only available if done "in order"
- may become extremely memory-intensive

Middle-ground: we parse object streams all at once. The rest is on-demand, and memoized.

## Reference parsing

Most objects are typed in advance, but the current Reference does not include that information.

Corollary: add a method on references to allows extraction. Possible signature:

```rust
impl<T> TypedRef<T> {
    // not a nom function: we don't need to consume input.
    // We know what we're expecting.
    fn extract(&self, reader: &mut impl Read, map: HashMap<usize, usize>) -> Result<T> {
        let &offset = map.get(self.object).unwrap();
        reader.seek(offset)?;
        // Some behaviour...
        return Ok(...)
    }
}
```
