---
title: Examples use panic! instead of proper error handling
severity: low
---

The example files (`examples/content.rs` and `examples/text_objects.rs`) use `panic!()` when parsing fails to consume the entire input:

```rust
if !stream.is_empty() {
    println!();
    println!("Issue while parsing");
    println!("{}", String::from_utf8_lossy(&stream[..500.min(stream.len())]));
    panic!("Parsing did not consume the entire input");
}
```

**Location**: 
- `examples/content.rs`
- `examples/text_objects.rs`

**Impact**: 
- Examples crash with a panic instead of returning an error
- Not idiomatic Rust (examples should demonstrate best practices)
- Makes it harder to use these examples as starting points for real applications

**Reproduction**: Run the examples with a PDF that has parsing issues.

**Suggested fix**: Return a `Result` from `main()` and use proper error handling with `eprintln!()` and `std::process::exit(1)` or propagate errors properly. The examples should demonstrate how to handle parsing errors gracefully.
