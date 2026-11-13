//! Compile-fail tests for the `livre-derive` macros, one file per case in `tests/ui/`.
//!
//! Re-snapshot the expected `.stderr` files with `TRYBUILD=overwrite`.

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
