use crate::{
    error::ParsingError,
    utilities::{take_eol_no_r, take_whitespace},
};
use nom::{
    branch::alt,
    bytes::complete::tag,
    combinator::map,
    error::{Error, ErrorKind, ParseError},
    sequence::delimited,
    Err, IResult,
};
use strum::IntoStaticStr;

use super::{
    stream::Stream, Array, Boolean, Dictionary, HexString, Integer, LiteralString, Name, Real,
};

#[derive(Debug, Clone, PartialEq, IntoStaticStr)]
pub enum Object {
    Null,
    Boolean(Boolean),
    Integer(Integer),
    Real(Real),
    LiteralString(LiteralString),
    HexString(HexString),
    Name(Name),
    Array(Array),
    Dictionary(Dictionary),
    Stream(Stream),
    // Reference(Reference),
}

impl Object {
    fn parse_stream_or_dict(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, dict) = Dictionary::parse(input)?;
        let Ok((input, _)) = delimited(take_whitespace, tag(b"stream"), take_eol_no_r)(input)
        else {
            return Ok((input, Self::Dictionary(dict)));
        };
        let (input, stream) = Stream::parse_with_dict(input, dict)?;

        Ok((input, Self::Stream(stream)))
    }

    /// Parse a single PDF object.
    ///
    /// Note that PDF objects are *not* delimited by `obj` and `endobj`.
    /// Such a case denotes a reference, which is not the same thing.
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        // Necessary in case we apply many0.
        if input.is_empty() {
            return Err(Err::Error(Error::from_error_kind(
                input,
                ErrorKind::NonEmpty,
            )));
        }

        // The order matters!
        // For instance, we should test `Real` before we test `Integer`,
        // and reference objects before numerics.
        let (input, obj) = alt((
            map(tag(b"null"), |_| Self::Null),
            map(Boolean::parse, Self::Boolean),
            map(Real::parse, Self::Real),
            map(Integer::parse, Self::Integer),
            map(LiteralString::parse, Self::LiteralString),
            Self::parse_stream_or_dict,
            map(HexString::parse, Self::HexString),
            map(Name::parse, Self::Name),
            map(Array::parse, Self::Array),
        ))(input)?;

        let (input, _) = take_whitespace(input)?;

        Ok((input, obj))
    }
}

macro_rules! try_into {
    ($into:ident) => {
        impl TryFrom<Object> for $into {
            type Error = ParsingError;

            fn try_from(value: Object) -> Result<$into, Self::Error> {
                match value {
                    Object::$into(item) => Ok(item.into()),
                    _ => Err(ParsingError::UnexpectedType {
                        expected: stringify!($into),
                        got: value.into(),
                    }),
                }
            }
        }
        impl From<$into> for Object {
            fn from(value: $into) -> Self {
                Self::$into(value)
            }
        }
    };
    ($into:ty => $via:ty) => {
        impl TryFrom<Object> for $into {
            type Error = ParsingError;

            fn try_from(value: Object) -> Result<$into, Self::Error> {
                let item: $via = value.try_into()?;
                let res = item.try_into()?;
                Ok(res)
            }
        }
    };
}

try_into!(Boolean);
try_into!(bool => Boolean);

try_into!(Integer);
try_into!(i8 => Integer);
try_into!(i16 => Integer);
try_into!(i32 => Integer);
try_into!(i64 => Integer);
try_into!(u8 => Integer);
try_into!(u16 => Integer);
try_into!(u32 => Integer);
try_into!(u64 => Integer);
try_into!(usize => Integer);

try_into!(Real);
try_into!(f32 => Real);

try_into!(LiteralString);
try_into!(String => LiteralString);

try_into!(Name);

try_into!(HexString);
try_into!(Vec<u8> => HexString);

try_into!(Array);
try_into!(Vec<Object> => Array);

#[macro_export]
macro_rules! obj {
    () => {
        Object::Null
    };
    (b:$val:literal) => {
        Object::Boolean(Boolean($val))
    };
    (i:$val:literal) => {
        Object::Integer(Integer($val))
    };
    (r:$val:literal) => {
        Object::Real(Real($val))
    };
    (t:$val:literal) => {
        Object::LiteralString(LiteralString($val.to_string()))
    };
    (h:$val:tt) => {
        Object::HexString(HexString($val.to_vec()))
    };
    (n:$val:literal) => {
        Object::Name(Name($val.to_string()))
    };
    ($($o:expr),+ $(,)?) => {
        Object::Array(vec![$($o),+].into())
    };
    ($($k:literal $v:expr),+ $(,)?) => {
        Object::Dictionary(Dictionary(vec![$(($k.to_string(), $v)),+].into_iter().collect()))
    };
    (s:$val:literal) => {
        Object::Stream(Stream{stream: $val.to_vec(), filters: Vec::new()})
    };
    (s:$val:literal | $filters:tt) => {
        Object::Stream(Stream{stream: $val.to_vec(), filters: $filters})
    };
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn parse(input: &[u8]) -> Object {
        let (_, obj) = Object::parse(input).unwrap();
        obj
    }

    #[rstest]
    #[case(obj!(b:true), Boolean(true))]
    #[case(obj!(i:-28), Integer(-28))]
    #[case(obj!(r:25.6), Real(25.6))]
    #[case(obj!(t:"test"), LiteralString("test".into()))]
    #[case(obj!(n:"test"), Name("test".into()))]
    #[case(obj!(h:[144, 31, 163]), HexString([144, 31, 163].into()))]
    fn obj_macro(#[case] input: Object, #[case] res: impl Into<Object>) {
        let res = res.into();
        assert_eq!(input, res)
    }

    #[allow(clippy::bool_assert_comparison)]
    #[rstest]
    #[case(b"null", obj!())]
    #[case(b"true", obj!(b:true))]
    #[case(b"false", obj!(b:false))]
    #[case(b"10", obj!(i:10))]
    #[case(b"-1023", obj!(i:-1023))]
    #[case(b"-.023", obj!(r:-0.023))]
    #[case(b"(a literal string)", obj!(t:"a literal string"))]
    #[case(b"<901FA>", obj!(h:[144, 31, 160]))]
    #[case(b"/TestName", obj!(n:"TestName"))]
    #[case(b"[1 2 true ]", obj![obj!(i:1), obj!(i:2), obj!(b:true)])]
    #[case(b"<</Length 9>>\nstream\n123456789\nendstream", obj!(s:b"123456789"))]
    fn test_parse(#[case] input: &[u8], #[case] res: Object) {
        assert_eq!(parse(input), res);
    }
}
