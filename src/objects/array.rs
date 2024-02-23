use nom::{multi::many0, IResult};

use crate::utilities::{take_whitespace, take_within_balanced};

use super::object::Object;

/// Represents a boolean within a PDF.
#[derive(Debug, PartialEq, Clone)]
pub struct Array(pub Vec<Object>);

impl Array {
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, value) = take_within_balanced(b'[', b']')(input)?;

        // We need to remove preceding whitespace.
        let (value, _) = take_whitespace(value)?;
        let (r, array) = many0(Object::parse)(value)?;

        assert!(
            r.is_empty(),
            "Remainder is not empty: {:?}",
            String::from_utf8_lossy(r)
        );
        Ok((input, Self(array)))
    }
}

impl From<Array> for Vec<Object> {
    fn from(value: Array) -> Self {
        value.0
    }
}

impl From<Vec<Object>> for Array {
    fn from(value: Vec<Object>) -> Self {
        Self(value)
    }
}

impl From<&[Object]> for Array {
    fn from(value: &[Object]) -> Self {
        Self(value.into())
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Boolean, Integer};

    use super::*;
    use rstest::rstest;

    fn parse(input: &[u8]) -> Array {
        let (_, obj) = Array::parse(input).unwrap();
        obj
    }

    #[allow(clippy::bool_assert_comparison)]
    #[rstest]
    #[case(b"[1 true]", &[Integer(1).into(), Boolean(true).into()])]
    #[case(b"[ false -1\n]", &[Boolean(false).into(), Integer(-1).into()])]
    fn test_parse(#[case] input: &[u8], #[case] result: &[Object]) {
        assert_eq!(parse(input), result.into());
    }
}
