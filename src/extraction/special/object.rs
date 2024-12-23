use winnow::{
    combinator::{alt, fail, peek},
    dispatch,
    token::any,
    BStr, PResult, Parser,
};

use crate::extraction::{extract, Extract};

use super::{
    map::Map,
    name::Name,
    reference::Reference,
    stream::Stream,
    strings::{HexadecimalString, LiteralString},
};

/// A type that can represent *any* PDF object.
///
/// Mainly for testing purposes, since the whole point of Livre is to provide a type-safe
/// implementation for PDF extraction.
///
/// Because of the stated goal, Livre's implementation for PDF objects **owns its data**
/// for simplicity.
///
/// From the specification:
///
/// > PDF syntax includes nine basic types of objects: boolean values, integers, real numbers,
/// > strings, names, arrays, dictionaries, streams, and the null object. Objects may be labelled
/// > so that they can be referred to by other objects. A labelled object is called an indirect object
/// > (see 7.3.10, "Indirect objects")
#[derive(Debug, Clone, PartialEq)]
pub enum Object {
    Null,
    Boolean(bool),
    Integer(i32),
    Real(f32),
    String(Vec<u8>),
    Name(Vec<u8>),
    Array(Vec<Object>),
    Dictionary(Map<Object>),
    Stream(Stream<Map<Object>>),
    Indirect(Reference<Object>),
}

impl From<()> for Object {
    fn from(_: ()) -> Self {
        Self::Null
    }
}

impl From<i32> for Object {
    fn from(value: i32) -> Self {
        Self::Integer(value)
    }
}

impl From<f32> for Object {
    fn from(value: f32) -> Self {
        Self::Real(value)
    }
}

impl From<LiteralString<'_>> for Object {
    fn from(LiteralString(inner): LiteralString) -> Self {
        Self::String(inner.to_vec())
    }
}

impl From<Name> for Object {
    fn from(Name(inner): Name) -> Self {
        Self::Name(inner)
    }
}

impl<T> From<Vec<T>> for Object
where
    Object: From<T>,
{
    fn from(value: Vec<T>) -> Self {
        let array = value.into_iter().map(Object::from).collect();
        Self::Array(array)
    }
}

impl<T> From<Map<T>> for Object
where
    Object: From<T>,
{
    fn from(value: Map<T>) -> Self {
        let map = value
            .into_iter()
            .map(|(k, v)| (k, Object::from(v)))
            .collect();
        Self::Dictionary(map)
    }
}

impl<K, V> FromIterator<(K, V)> for Object
where
    Name: From<K>,
    Object: From<V>,
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let map = iter
            .into_iter()
            .map(|(k, v)| (Name::from(k), Object::from(v)))
            .collect();
        Self::Dictionary(map)
    }
}

impl From<Stream<Map<Object>>> for Object {
    fn from(value: Stream<Map<Object>>) -> Self {
        Self::Stream(value)
    }
}

impl From<Stream<()>> for Object {
    fn from(value: Stream<()>) -> Self {
        let Stream { content, .. } = value;
        Self::Stream(Stream {
            structured: Map::new(),
            content,
        })
    }
}

impl From<bool> for Object {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}

impl From<String> for Object {
    fn from(value: String) -> Self {
        Self::String(value.into())
    }
}

impl From<&str> for Object {
    fn from(value: &str) -> Self {
        Self::String(value.into())
    }
}

impl From<HexadecimalString> for Object {
    fn from(HexadecimalString(value): HexadecimalString) -> Self {
        Self::String(value)
    }
}

impl From<Reference<Object>> for Object {
    fn from(value: Reference<Object>) -> Self {
        Self::Indirect(value)
    }
}

impl Extract<'_> for Object {
    fn extract(input: &mut &BStr) -> PResult<Self> {
        dispatch! {peek(any);
            b'n' => b"null".map(|_| Object::Null),
            b't' | b'f' => bool::extract.map(Object::Boolean),
            b'0'..=b'9' => alt((Reference::extract.map(Object::from), number)),
            b'+' | b'-' => number,
            b'(' => LiteralString::extract.map(Object::from),
            b'/' => Name::extract.map(Object::from),
            b'<' => alt((map_or_stream, HexadecimalString::extract.map(Object::from))),
            b'[' => Vec::<Object>::extract.map(Object::Array),
            _ => fail,
        }
        .parse_next(input)
    }
}

fn number(input: &mut &BStr) -> PResult<Object> {
    alt((
        f32::extract
            .with_taken()
            .verify_map(|(x, bytes)| bytes.contains(&b'.').then_some(x))
            .map(Object::from),
        i32::extract.map(Object::from),
    ))
    .parse_next(input)
}

fn map_or_stream(input: &mut &BStr) -> PResult<Object> {
    // TODO: avoid parsing twice. This will do for now.
    alt((
        extract::<Stream<Map<Object>>>.map(Object::Stream),
        Map::<Object>::extract.map(Object::Dictionary),
    ))
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case((), Object::Null)]
    #[case(true, Object::Boolean(true))]
    #[case(42, Object::Integer(42))]
    #[case(-42.0, Object::Real(-42.0))]
    #[case("test".to_string(), Object::String("test".into()))]
    #[case(Name::from("test"), Object::Name(vec![0x74, 0x65, 0x73, 0x74]))]
    #[case(vec![0, 0], Object::Array(vec![Object::Integer(0), Object::Integer(0)]))]
    fn into_object<T>(#[case] v: T, #[case] expected: Object)
    where
        T: Into<Object>,
    {
        assert_eq!(expected, v.into())
    }

    #[rstest]
    #[case(b"null", ())]
    #[case(b"1", 1i32)]
    #[case(b"1.0", 1.0f32)]
    #[case(b"[true 1]", vec![Object::Boolean(true), Object::Integer(1)])]
    #[case(b"(test)", "test")]
    #[case(b"/test", Name::from("test"))]
    #[case(
        indoc!{b"
            <<
            /bool true
            /int 1
            >>
        "},
        vec![("bool", Object::Boolean(true)), ("int", Object::Integer(1))].into_iter().collect::<Object>()
    )]
    #[case(b"0 0 R", Reference::<Object>::from((0, 0)))]
    #[case(
        indoc!{b"
            <</Length 1>>stream
            0
            endstream
        "},
        Stream {
            content: b"0".into(),
            structured: (),
        }
    )]
    #[case(
        indoc!{b"
            <</Length 1/Test (test)>>stream
            0
            endstream
        "},
        Stream::<Map<Object>> {
            content: b"0".into(),
            structured: vec![
                (Name::from("Test"), Object::from("test"))
            ].into_iter().collect(),
        }
    )]
    fn parse_object(#[case] input: &[u8], #[case] expected: impl Into<Object>) {
        let expected: Object = expected.into();
        let result = Object::extract.parse_next(&mut input.into()).unwrap();

        assert_eq!(expected, result);
    }
}
