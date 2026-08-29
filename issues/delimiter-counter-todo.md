---
title: TODO: Evaluate need for u16 counter in delimiter parsing
severity: low
---

In `src/extraction/utilities/delimiter.rs`, there's a TODO comment about the counter type used for tracking nesting levels:

```rust
// A `u8` counter would probably be amply sufficient already...
// It seems unlikely to go beyond a few level of nesting, let alone 256.
// Let us remain on the side of caution for now.
// TODO: evaluate the need for a `u16` here.
let mut counter: u16 = 1;
```

**Location**: `src/extraction/utilities/delimiter.rs`

**Impact**: The code uses a `u16` counter for tracking nesting levels in delimiter parsing (likely for balanced delimiters like `<<>>` or `(())`). The comment suggests this is overly cautious and a `u8` (max 255) would be sufficient.

**Reproduction**: Inspect the delimiter parsing code.

**Suggested fix**: 
1. Evaluate the maximum realistic nesting level in PDF documents
2. If `u8` is sufficient (which the comment suggests), change the counter type to `u8`
3. Remove the TODO comment once the decision is made

This is a minor optimization opportunity that could reduce memory usage slightly.
