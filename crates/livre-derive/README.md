# Derive macros for `livre` crate

`livre-derive` provides the `Extract`, `FromRawDict` and `BuildFromRawDict`
derives for [`livre`](https://crates.io/crates/livre), a type-safe PDF parser.
Use them through `livre`, which re-exports each next to its trait:

```rust,ignore
use livre::extraction::{FromRawDict, MaybeArray};

#[derive(FromRawDict)]
struct StreamConfig {
    // Reads `/Length`.
    length: usize,
    // `/Filter` may be one filter or an array; absent means none.
    #[livre(from = MaybeArray<Filter>, default)]
    filter: Vec<Filter>,
    #[livre(rename = "DL")]
    decoded_length: Option<usize>,
}
```

The supported shapes, the `rename`, `from`, `default` and `flatten`
attributes, and the key convention are documented on
[docs.rs](https://docs.rs/livre-derive).

The minimum supported Rust version is 1.85.
