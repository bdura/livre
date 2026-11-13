//! Behavioural tests for the `livre-derive` macros, compiled as a downstream crate.

use std::collections::HashMap;

use livre::{
    extraction::{extract, Extract, FromRawDict, MaybeArray, Reference, ReferenceId},
    follow_refs::{Build, BuildFromRawDict, Builder},
};
use winnow::{
    error::{ContextError, ErrMode, StrContext},
    BStr, ModalResult,
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

/// A `from` type without a `Default` impl, converting into `Vec<i32>`.
///
/// Proves that `from` + `default` defaults the field, not the `from` type.
#[derive(Debug, PartialEq)]
struct NoDefault(Vec<i32>);

impl<'de> Extract<'de> for NoDefault {
    fn extract(input: &mut &'de BStr) -> ModalResult<Self> {
        MaybeArray::<i32>::extract(input).map(|m| Self(m.0))
    }
}

impl Build for NoDefault {
    fn build<B: Builder>(input: &mut &BStr, _: &B) -> ModalResult<Self> {
        Self::extract(input)
    }
}

impl From<NoDefault> for Vec<i32> {
    fn from(value: NoDefault) -> Self {
        value.0
    }
}

fn compute() -> f32 {
    3.0
}

/// The `StrContext::Label`s attached to an error, innermost first.
fn labels(err: &ContextError) -> Vec<&'static str> {
    err.context()
        .filter_map(|c| match c {
            StrContext::Label(label) => Some(*label),
            _ => None,
        })
        .collect()
}

/// The behavioural matrix shared by both dictionary derives.
///
/// Instantiated once per derive: `$derive` names the derive, `$bound` the
/// trait a flattened type parameter must implement, and `parse` reads a
/// dictionary through the derived impl.
macro_rules! dict_matrix {
    ($module:ident, $derive:path, [$($bound:tt)*], $parse:item) => {
        mod $module {
            use super::*;
            use rstest::rstest;

            $parse

            #[derive(Debug, PartialEq, $derive)]
            struct Required {
                length: i32,
            }

            #[test]
            fn required_key_present() {
                assert_eq!(parse::<Required>(b"<</Length 4>>").unwrap(), Required { length: 4 });
            }

            #[test]
            fn required_key_absent() {
                assert!(parse::<Required>(b"<</Other 4>>").is_err());
            }

            #[derive(Debug, PartialEq, $derive)]
            struct Optional {
                length: Option<i32>,
            }

            #[rstest]
            #[case(b"<</Length 4>>", Some(4))]
            #[case(b"<<>>", None)]
            #[case(b"<</Length null>>", None)]
            fn option_field(#[case] input: &'static [u8], #[case] expected: Option<i32>) {
                assert_eq!(parse::<Optional>(input).unwrap(), Optional { length: expected });
            }

            #[derive(Debug, PartialEq, $derive)]
            struct Renamed {
                #[livre(rename = "ID")]
                id: i32,
            }

            #[test]
            fn rename() {
                assert_eq!(parse::<Renamed>(b"<</ID 7>>").unwrap(), Renamed { id: 7 });
                assert!(parse::<Renamed>(b"<</Id 7>>").is_err());
            }

            #[derive(Debug, PartialEq, $derive)]
            struct FromArray {
                #[livre(from = MaybeArray<i32>)]
                values: Vec<i32>,
            }

            #[rstest]
            #[case(b"<</Values 1>>", vec![1])]
            #[case(b"<</Values [1 2]>>", vec![1, 2])]
            fn from_maybe_array(#[case] input: &'static [u8], #[case] expected: Vec<i32>) {
                assert_eq!(parse::<FromArray>(input).unwrap(), FromArray { values: expected });
            }

            #[derive(Debug, PartialEq, $derive)]
            struct OptionalFrom {
                #[livre(from = MaybeArray<i32>)]
                values: Option<Vec<i32>>,
            }

            #[rstest]
            #[case(b"<</Values [1 2]>>", Some(vec![1, 2]))]
            #[case(b"<</Values 1>>", Some(vec![1]))]
            #[case(b"<<>>", None)]
            fn from_on_option_field(
                #[case] input: &'static [u8],
                #[case] expected: Option<Vec<i32>>,
            ) {
                assert_eq!(
                    parse::<OptionalFrom>(input).unwrap(),
                    OptionalFrom { values: expected }
                );
            }

            #[derive(Debug, PartialEq, $derive)]
            struct FromDefault {
                #[livre(from = NoDefault, default)]
                values: Vec<i32>,
            }

            #[rstest]
            #[case(b"<<>>", vec![])]
            #[case(b"<</Values [1 2]>>", vec![1, 2])]
            fn from_then_default(#[case] input: &'static [u8], #[case] expected: Vec<i32>) {
                assert_eq!(parse::<FromDefault>(input).unwrap(), FromDefault { values: expected });
            }

            #[derive(Debug, PartialEq, $derive)]
            struct Defaults {
                #[livre(default)]
                bare: f32,
                #[livre(default = 1.0)]
                lit: f32,
                #[livre(default = || 2.0)]
                closure: f32,
                #[livre(default = compute())]
                expr: f32,
            }

            #[rstest]
            #[case(b"<<>>", Defaults { bare: 0.0, lit: 1.0, closure: 2.0, expr: 3.0 })]
            #[case(
                b"<</Bare 5 /Lit 6 /Closure 7 /Expr 8>>",
                Defaults { bare: 5.0, lit: 6.0, closure: 7.0, expr: 8.0 },
            )]
            fn defaults(#[case] input: &'static [u8], #[case] expected: Defaults) {
                assert_eq!(parse::<Defaults>(input).unwrap(), expected);
            }

            #[derive(Debug, PartialEq, $derive)]
            struct First {
                a: i32,
            }

            #[derive(Debug, PartialEq, $derive)]
            struct Second {
                b: i32,
            }

            #[derive(Debug, PartialEq, $derive)]
            struct Concrete {
                #[livre(flatten)]
                first: First,
                c: i32,
            }

            #[test]
            fn flatten_concrete() {
                assert_eq!(
                    parse::<Concrete>(b"<</A 1 /C 3>>").unwrap(),
                    Concrete { first: First { a: 1 }, c: 3 }
                );
            }

            #[derive(Debug, PartialEq, $derive)]
            struct Shared {
                #[livre(flatten)]
                first: First,
                #[livre(flatten)]
                second: Second,
            }

            #[test]
            fn flatten_two_share_one_dict() {
                assert_eq!(
                    parse::<Shared>(b"<</B 2 /A 1>>").unwrap(),
                    Shared { first: First { a: 1 }, second: Second { b: 2 } }
                );
            }

            #[derive(Debug, PartialEq, $derive)]
            struct Generic<T> {
                #[livre(flatten)]
                inner: T,
                c: i32,
            }

            #[test]
            fn flatten_type_parameter() {
                assert_eq!(
                    parse::<Generic<First>>(b"<</A 1 /C 3>>").unwrap(),
                    Generic { inner: First { a: 1 }, c: 3 }
                );
            }

            #[derive(Debug, PartialEq, $derive)]
            struct Len {
                length: usize,
            }

            #[test]
            fn missing_key_is_labelled_backtrack() {
                let err = parse::<Len>(b"<</Foo 1>>").unwrap_err();
                let ErrMode::Backtrack(err) = err else {
                    panic!("expected a backtrack, got {err:?}");
                };
                assert_eq!(labels(&err), ["/Length"]);
            }

            #[test]
            fn failing_value_is_labelled() {
                let err = parse::<Len>(b"<</Length /NotANumber>>").unwrap_err();
                let err = err.into_inner().expect("a complete error");
                assert!(labels(&err).contains(&"/Length"));
            }

            #[test]
            fn flatten_adds_no_label() {
                let err = parse::<Concrete>(b"<</C 3>>").unwrap_err();
                let err = err.into_inner().expect("a complete error");
                assert_eq!(labels(&err), ["/A"]);
            }

            // Compiles only if `Generic<T>` requires no more of `T` than `$bound`.
            #[allow(dead_code)]
            fn flattened_param_is_bounded_by_the_dict_trait<T: $($bound)*>() {
                fn implements<U: $($bound)*>() {}
                implements::<Generic<T>>();
            }
        }
    };
}

dict_matrix!(
    from_raw_dict_matrix,
    FromRawDict,
    [FromRawDict<'static>],
    fn parse<T: FromRawDict<'static>>(input: &'static [u8]) -> ModalResult<T> {
        T::extract(&mut input.as_ref())
    }
);

dict_matrix!(
    build_from_raw_dict_matrix,
    BuildFromRawDict,
    [BuildFromRawDict],
    fn parse<T: BuildFromRawDict>(input: &'static [u8]) -> ModalResult<T> {
        T::build(&mut input.as_ref(), &())
    }
);

#[derive(Debug, PartialEq, BuildFromRawDict)]
struct Target {
    value: i32,
}

#[derive(Debug, PartialEq, BuildFromRawDict)]
struct Referring {
    target: Reference<Target>,
}

#[test]
fn build_resolves_reference_through_builder() {
    let mut builder: HashMap<ReferenceId, &BStr> = HashMap::new();
    builder.insert(
        ReferenceId::new(1, 0),
        b"1 0 obj <</Value 42>> endobj".as_slice().as_ref(),
    );

    let referring =
        Referring::build(&mut b"<</Target 1 0 R>>".as_slice().as_ref(), &builder).unwrap();
    let target = builder.build_reference(referring.target).unwrap();

    assert_eq!(target, Target { value: 42 });
}

/// The `Extract` expansion must not rely on the caller importing `winnow::Parser`.
mod extract_without_parser_import {
    use livre::extraction::Extract;
    use winnow::BStr;

    fn parse<'de, T: Extract<'de>>(bytes: &'de [u8]) -> winnow::ModalResult<T> {
        let mut input: &'de BStr = bytes.as_ref();
        T::extract(&mut input)
    }

    #[derive(Debug, PartialEq, Extract)]
    struct Unit;

    #[derive(Debug, PartialEq, Extract)]
    struct IntBool(i32, bool);

    #[derive(Debug, PartialEq, Extract)]
    struct Wrap<T>(T);

    #[derive(Debug, PartialEq, Extract)]
    enum Either {
        Int(i32),
        Flag(bool),
    }

    #[derive(Debug, PartialEq, Extract)]
    enum IntOrNothing {
        Int(i32),
        Nothing(Unit),
    }

    #[test]
    fn unit_struct() {
        assert_eq!(parse::<Unit>(b"").unwrap(), Unit);
    }

    #[test]
    fn tuple_struct() {
        assert_eq!(parse::<IntBool>(b"1 true").unwrap(), IntBool(1, true));
    }

    #[test]
    fn generic_tuple_struct() {
        assert_eq!(parse::<Wrap<i32>>(b"3").unwrap(), Wrap(3));
    }

    #[test]
    fn enum_first_variant() {
        assert_eq!(parse::<Either>(b"7").unwrap(), Either::Int(7));
    }

    #[test]
    fn enum_second_variant() {
        assert_eq!(parse::<Either>(b"false").unwrap(), Either::Flag(false));
    }

    #[test]
    fn enum_no_variant_matches() {
        assert!(parse::<Either>(b"/Name").is_err());
    }

    #[test]
    fn enum_alt_over_unit_extractor() {
        assert_eq!(parse::<IntOrNothing>(b"5").unwrap(), IntOrNothing::Int(5));
        assert_eq!(
            parse::<IntOrNothing>(b"/Name").unwrap(),
            IntOrNothing::Nothing(Unit)
        );
    }
}
