use crate::{error::ParsingError, utilities::take_whitespace};
use nom::{
    branch::alt,
    combinator::map,
    error::{Error, ErrorKind, ParseError},
    Err, IResult,
};
use strum::IntoStaticStr;

use super::{Boolean, Integer};

#[derive(Debug, Clone, PartialEq, IntoStaticStr)]
pub enum Object {
    // Null,
    Boolean(Boolean),
    Integer(Integer),
}

impl Object {
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

        let (input, obj) = alt((
            map(Boolean::parse, Self::Boolean),
            map(Integer::parse, Self::Integer),
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
    };
    ($into:ident via $via:ident) => {
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
try_into!(bool via Boolean);

try_into!(Integer);
try_into!(i32 via Integer);

#[macro_export]
macro_rules! obj {
    (b:$val:literal) => {
        Object::Boolean(Boolean($val))
    };
    (i:$val:literal) => {
        Object::Integer(Integer($val))
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
    #[case(obj!(b:true), Object::Boolean(Boolean(true)))]
    #[case(obj!(i:-28), Object::Integer(Integer(-28)))]
    fn obj_macro(#[case] input: Object, #[case] res: Object) {
        assert_eq!(input, res)
    }

    #[allow(clippy::bool_assert_comparison)]
    #[rstest]
    #[case(b"true", obj!(b:true))]
    #[case(b"false", obj!(b:false))]
    #[case(b"10", obj!(i:10))]
    #[case(b"-1023", obj!(i:-1023))]
    fn boolean(#[case] input: &[u8], #[case] res: Object) {
        assert_eq!(parse(input), res);
    }
}
