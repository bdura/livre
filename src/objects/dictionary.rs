use std::collections::HashMap;

use nom::{
    error::{Error, ErrorKind, ParseError},
    multi::many0,
    Err, IResult,
};

use crate::{
    objects::name::Name,
    utilities::{take_whitespace, take_within_balanced},
};

use super::object::Object;

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Dictionary(pub HashMap<String, Object>);

impl Dictionary {
    fn parse_key_value(input: &[u8]) -> IResult<&[u8], (String, Object)> {
        let (input, Name(key)) = Name::parse(input)?;
        let (input, _) = take_whitespace(input)?;
        let (input, obj) = Object::parse(input)?;
        Ok((input, (key, obj)))
    }

    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        // dictionaries are enclosed by double angle brackets.
        let (input, value) = take_within_balanced(b'<', b'>')(input)?;
        let (d, value) = take_within_balanced(b'<', b'>')(value)?;

        if !d.is_empty() {
            return Err(Err::Error(Error::from_error_kind(
                input,
                ErrorKind::TakeTill1,
            )));
        }

        let (value, _) = take_whitespace(value)?;
        let (r, array) = many0(Self::parse_key_value)(value)?;

        assert!(
            r.is_empty(),
            "Remainder is not empty: {:?}",
            String::from_utf8_lossy(r)
        );

        Ok((input, Self(array.into_iter().collect())))
    }
}

impl From<Dictionary> for HashMap<String, Object> {
    fn from(value: Dictionary) -> Self {
        value.0
    }
}

impl From<HashMap<String, Object>> for Dictionary {
    fn from(value: HashMap<String, Object>) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Boolean, Integer};

    use super::*;
    use rstest::rstest;

    fn parse(input: &[u8]) -> Dictionary {
        let (_, obj) = Dictionary::parse(input).unwrap();
        obj
    }

    #[allow(clippy::bool_assert_comparison)]
    #[rstest]
    #[case(b"<</Type 1>>", &[("Type".into(), Integer(1).into())])]
    #[case(b"<</k1 1/k2 true>>", &[("k1".into(), Integer(1).into()), ("k2".into(), Boolean(true).into())])]
    fn test_parse(#[case] input: &[u8], #[case] result: &[(String, Object)]) {
        let result: HashMap<String, Object> = result.iter().cloned().collect();
        assert_eq!(parse(input), Dictionary(result));
    }
}
