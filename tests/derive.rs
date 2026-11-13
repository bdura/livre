//! Behavioural tests for the `livre-derive` macros, compiled as a downstream crate.

use livre::{
    extraction::{extract, Extract, FromRawDict},
    follow_refs::{Build, BuildFromRawDict},
};

#[derive(Debug, PartialEq, Extract)]
struct Pair(i32, i32);

#[derive(Debug, PartialEq, FromRawDict)]
struct Len {
    length: usize,
}

#[derive(Debug, PartialEq, BuildFromRawDict)]
struct BuiltLen {
    length: usize,
}

#[test]
fn extract_tuple_struct() {
    assert_eq!(
        extract::<Pair>(&mut b"1 2".as_slice().as_ref()).unwrap(),
        Pair(1, 2)
    );
}

#[test]
fn from_raw_dict_named_struct() {
    assert_eq!(
        Len::extract(&mut b"<</Length 42>>".as_slice().as_ref()).unwrap(),
        Len { length: 42 }
    );
}

#[test]
fn build_from_raw_dict_named_struct() {
    assert_eq!(
        BuiltLen::build(&mut b"<</Length 42>>".as_slice().as_ref(), &()).unwrap(),
        BuiltLen { length: 42 }
    );
}
