//! Compile-fail tests, one file per case in `tests/ui/`: the `livre-derive` macros and the
//! coordinate spaces of `livre::content`.
//!
//! Re-snapshot the expected `.stderr` files with `TRYBUILD=overwrite`.

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
