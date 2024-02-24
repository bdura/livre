use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use nom::{
    error::{Error, ErrorKind, ParseError},
    multi::many0,
    Err, IResult,
};

use crate::{
    error::{ParsingError, Result},
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

    pub fn pop<T, E>(&mut self, key: &str) -> Result<T>
    where
        T: TryFrom<Object, Error = E>,
        ParsingError: From<E>,
    {
        let result = self
            .remove(key)
            .ok_or_else(|| ParsingError::KeyNotFound(key.into()))?
            .try_into()?;

        Ok(result)
    }

    pub fn pop_opt<T, E>(&mut self, key: &str) -> Result<Option<T>>
    where
        T: TryFrom<Object, Error = E>,
        ParsingError: From<E>,
    {
        let result = self.remove(key).map(|obj| T::try_from(obj)).transpose()?;
        Ok(result)
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

impl Deref for Dictionary {
    type Target = HashMap<String, Object>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Dictionary {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
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
