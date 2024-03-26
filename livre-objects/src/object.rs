use std::collections::HashMap;

use livre_extraction::{Extract, RawDict};
use livre_utilities::{recognize_number, take_whitespace};
use nom::{
    branch::alt,
    bytes::complete::tag,
    combinator::{map, peek},
    error::Error,
    IResult, ParseTo,
};

use super::Stream;
use crate::{HexBytes, Name, Reference};

type ObjectMap<'input> = HashMap<String, Object<'input>>;

#[derive(Debug, Clone, PartialEq)]
pub enum Object<'input> {
    Null,
    Boolean(bool),
    Integer(i32),
    Real(f32),
    LiteralString(String),
    HexString(HexBytes),
    Name(String),
    Array(Vec<Object<'input>>),
    Dictionary(HashMap<String, Object<'input>>),
    Stream(Stream<'input, ObjectMap<'input>>),
    Reference(Reference),
}

impl<'input> Object<'input> {
    fn extract_stream_or_dict(input: &'input [u8]) -> IResult<&'input [u8], Self> {
        let (input, dict) = RawDict::extract(input)?;
        let (input, _) = take_whitespace(input)?;

        let res = peek(tag::<&[u8], &[u8], Error<&'input [u8]>>(b"stream"))(input);

        if res.is_ok() {
            let (input, stream) =
                Stream::<'input, ObjectMap<'input>>::extract_from_dict(input, dict)?;
            let obj = Self::Stream(stream);
            Ok((input, obj))
        } else {
            let obj = Self::Dictionary(dict.convert().unwrap());
            Ok((input, obj))
        }
    }

    fn extract_numeric(input: &'input [u8]) -> IResult<&'input [u8], Self> {
        let (input, num) = recognize_number(input)?;
        if num.contains(&b'.') {
            let x: f32 = num
                .parse_to()
                .expect("By construction, the format is correct");
            Ok((input, Self::Real(x)))
        } else {
            let n: i32 = num
                .parse_to()
                .expect("By construction, the format is correct");
            Ok((input, Self::Integer(n)))
        }
    }
}

impl<'input> Extract<'input> for Object<'input> {
    fn extract(input: &'input [u8]) -> IResult<&'input [u8], Self> {
        alt((
            map(tag("null"), |_| Self::Null),
            map(bool::extract, Self::Boolean),
            Self::extract_stream_or_dict,
            map(Reference::extract, Self::Reference),
            Self::extract_numeric,
            map(Vec::<Object>::extract, Self::Array),
            map(HexBytes::extract, Self::HexString),
            map(String::extract, Self::LiteralString),
            map(Name::extract, |Name(name)| Self::Name(name)),
        ))(input)
    }
}

#[cfg(test)]
mod tests {

    use indoc::indoc;
    use livre_filters::{Filter, FlateDecode};
    use rstest::rstest;

    use super::*;

    macro_rules! obj {
        () => {
            Object::Null
        };
        (b:$val:literal) => {
            Object::Boolean($val)
        };
        (i:$val:literal) => {
            Object::Integer($val)
        };
        (f:$val:literal) => {
            Object::Real($val)
        };
        (t:$val:literal) => {
            Object::LiteralString($val.to_string())
        };
        (h:$val:tt) => {
            Object::HexString(HexBytes($val.to_vec()))
        };
        (n:$val:literal) => {
            Object::Name($val.to_string())
        };
        ($($o:expr),+ $(,)?) => {
            Object::Array(vec![$($o),+].into())
        };
        (r:$obj:literal $gen:literal) => {
            Object::Reference(Reference{object: $obj, generation: $gen})
        };
        ($($k:literal $v:expr),+ $(,)?) => {
            Object::Dictionary(vec![$(($k.to_string(), $v)),+].into_iter().collect())
        };
        (s:$val:literal) => {
            Object::Stream(Stream{inner: $val, filters: Vec::new(), structured: Default::default()})
        };
        (s:$val:literal | $filters:expr) => {
            Object::Stream(Stream{inner: $val, filters: $filters, structured: Default::default()})
        };
    }

    #[rstest]
    #[case(b"null", obj!())]
    #[case(b"true", obj!(b:true))]
    #[case(b"false", obj!(b:false))]
    #[case(b"0 10 R", obj!(r:0 10))]
    #[case(b"(Longtemps, je me suis)", obj!(t:"Longtemps, je me suis"))]
    #[case(b"<00A01>", obj!(h:[0, 160, 16]))]
    #[case(b"<00A010>", obj!(h:[0, 160, 16]))]
    #[case(b"<</FirstKey/Test/AnotherKey 2.>>", obj!("FirstKey" obj!(n:"Test"), "AnotherKey" obj!(f:2.0)))]
    #[case(indoc!{b"
        <<
            /DA(/Helv 0 Tf 0 g )
            /Fields[]
        >>
    "}, obj!("DA" obj!(t:"/Helv 0 Tf 0 g "), "Fields" Object::Array(vec![])))]
    #[case(
        indoc! {b"
            <</Length 10>> stream
            0123456789
            endstream
        "},
        obj!(s:b"0123456789")
    )]
    #[case(
        indoc! {b"
            <</Length 10/Filter/FlateDecode>> stream
            0123456789
            endstream
        "},
        obj!(s:b"0123456789" | vec![Filter::FlateDecode(FlateDecode)])
    )]
    fn object(#[case] input: &[u8], #[case] expected: Object) {
        let (_, obj) = Object::extract(input).unwrap();
        assert_eq!(obj, expected);
    }
}
