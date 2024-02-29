use std::collections::HashMap;

use livre_extraction::{
    complex::{Dictionary, RawDict},
    pdf_types::Reference,
    utilities::{take_eol_no_r, take_whitespace},
    Extract,
};
use nom::{bytes::complete::tag, error::ParseError, sequence::tuple, IResult};

use super::HexString;

#[derive(Debug, Clone, PartialEq)]
pub enum Object {
    Null,
    Boolean(bool),
    Integer(i32),
    Real(f32),
    LiteralString(String),
    HexString(HexString),
    Array(Vec<Object>),
    Dictionary(HashMap<String, Object>),
    Reference(Reference),
}

impl Object {
    fn parse_stream_or_dict(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, dict) = RawDict::extract(input)?;
        let Ok((input, _)) = tuple((take_whitespace, tag("stream"), take_eol_no_r))(input) else {
            let map = dict.convert().map_err(|_| {
                nom::Err::Error(nom::error::Error::from_error_kind(
                    input,
                    nom::error::ErrorKind::IsNot,
                ))
            })?;
            return Ok((input, Self::Dictionary(map)));
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
            map(tag("null"), |_| Self::Null),
            map(Boolean::parse, Self::Boolean),
            map(Reference::parse, Self::Reference),
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

    pub fn parse_indirect(input: &[u8]) -> IResult<&[u8], (Reference, Self)> {
        let (input, (object, _, generation, _)) =
            tuple((parse_digits, tag(" "), parse_digits, tag(" obj")))(input)?;
        let reference = Reference::new(object, generation);
        let (input, _) = take_whitespace1(input)?;
        let (input, obj) = Self::parse(input)?;
        let (input, _) = tag("endobj")(input)?;
        let (input, _) = take_whitespace1(input)?;

        Ok((input, (reference, obj)))
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

#[macro_export]
/// Macro that simplifies the creation of [`Object`] elements.
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
    (f:$val:literal) => {
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
    (r:$obj:literal $gen:literal) => {
        Object::Reference(Reference{object: $obj, generation: $gen})
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
    #[case(obj!(f:25.6), Real(25.6))]
    #[case(obj!(t:"test"), LiteralString("test".into()))]
    #[case(obj!(n:"test"), Name("test".into()))]
    #[case(obj!(h:[144, 31, 163]), HexString([144, 31, 163].into()))]
    #[case(obj!(r:0 0), Reference{object: 0, generation: 0})]
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
    #[case(b"-.023", obj!(f:-0.023))]
    #[case(b"(a literal string)", obj!(t:"a literal string"))]
    #[case(b"<901FA>", obj!(h:[144, 31, 160]))]
    #[case(b"/TestName", obj!(n:"TestName"))]
    #[case(b"[1 2 true ]", obj![obj!(i:1), obj!(i:2), obj!(b:true)])]
    #[case(b"<</Length 9>>\nstream\n123456789\nendstream", obj!(s:b"123456789"))]
    #[case(b"10 0 R", obj!(r:10 0))]
    fn test_parse(#[case] input: &[u8], #[case] res: Object) {
        assert_eq!(parse(input), res);
    }
}
