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

#[derive(Debug, PartialEq, FromRawDict)]
struct WithExprDefault {
    #[livre(default = Vec::new())]
    values: Vec<i32>,
}

#[test]
fn default_accepts_any_expression() {
    assert_eq!(
        WithExprDefault::extract(&mut b"<<>>".as_slice().as_ref()).unwrap(),
        WithExprDefault { values: Vec::new() }
    );
}

#[derive(Debug, PartialEq, FromRawDict)]
struct Weird {
    __weird: i32,
}

#[test]
fn key_skips_empty_segments() {
    assert_eq!(
        Weird::extract(&mut b"<</Weird 3>>".as_slice().as_ref()).unwrap(),
        Weird { __weird: 3 }
    );
}

#[derive(Debug, PartialEq, FromRawDict)]
struct Inner<T> {
    value: T,
}

#[derive(Debug, PartialEq, FromRawDict)]
struct FlattenGeneric {
    #[livre(flatten)]
    inner: Inner<i32>,
    other: i32,
}

#[test]
fn flatten_on_generic_type() {
    assert_eq!(
        FlattenGeneric::extract(&mut b"<</Value 1 /Other 2>>".as_slice().as_ref()).unwrap(),
        FlattenGeneric {
            inner: Inner { value: 1 },
            other: 2
        }
    );
}
