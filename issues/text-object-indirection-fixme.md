---
title: FIXME: Unnecessary indirection in TextObject text_buffer
severity: low
---

In `src/content/state/text.rs`, there's a FIXME comment indicating that the `text_buffer` field in `TextObject` uses unnecessary indirection:

```rust
// FIXME: this indirection will be needed down the line. For now it seems a bit dumb.
// It should be replaced with a `VecDeque<u8>` to allow the *font* to iterate over the text
pub text_buffer: Option<PDFString>,
```

**Location**: `src/content/state/text.rs`, around line 45

**Impact**: The current implementation uses `Option<PDFString>` for the text buffer, but the comment suggests this is an unnecessary indirection. The suggested improvement is to use `VecDeque<u8>` directly to allow fonts to iterate over the text more efficiently.

**Reproduction**: Inspect the `TextObject` struct definition.

**Suggested fix**: Replace `Option<PDFString>` with `VecDeque<u8>` as suggested in the FIXME comment. This would simplify the type and potentially improve performance for text iteration.
