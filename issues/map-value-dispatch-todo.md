---
title: TODO: Replace catch-all with per-type dispatch in Map value parsing
severity: low
---

In `src/extraction/special/map.rs`, there's a TODO comment about the catch-all case for parsing map values:

```rust
// Catch-all for values that start with bytes other than the dispatch arms above:
// numbers (e.g. `42`, `-3.14`), booleans (`true`, `false`), and indirect references
// (`1 0 R`). These all end at the next `/` (the start of the next key) or at `>>`.
// TODO: replace this with per-type dispatch once all value kinds are covered, so that
// truly invalid input is rejected rather than silently consumed.
_ => take_till(0.., b'/').map(remove_trailing_spaces),
```

**Location**: `src/extraction/special/map.rs`

**Impact**: The current implementation uses a catch-all that silently consumes any value starting with unexpected bytes, treating it as a string until the next `/` or `>>`. This means:
- Truly invalid input is not rejected
- Type information is lost (numbers, booleans, references are all treated as strings)
- Potential for parsing incorrect values that happen to look valid

**Reproduction**: Parse a PDF with a map containing a value that matches the catch-all pattern.

**Suggested fix**: Implement proper per-type dispatch for all value kinds (numbers, booleans, references) so that invalid input is properly rejected and types are preserved.
