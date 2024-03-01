use nom::{
    error::{Error, ErrorKind, ParseError},
    multi::many0,
    Err, IResult,
};
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use crate::{
    error::{self, Result},
    utilities::{take_whitespace, take_within_balanced},
};

use crate::extraction::{Extract, Parse};

mod utilities;
use utilities::parse_key_value;
pub use utilities::parse_name;

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Dictionary<T>(pub HashMap<String, T>);

pub type RawDict<'input> = Dictionary<&'input [u8]>;

impl<'input> Extract<'input> for HashMap<String, &'input [u8]> {
    fn extract(input: &'input [u8]) -> IResult<&'input [u8], Self> {
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
        let (r, array) = many0(parse_key_value)(value)?;

        assert!(
            r.is_empty(),
            "Remainder is not empty: {:?}",
            String::from_utf8_lossy(r)
        );

        let map: HashMap<String, &[u8]> = array.into_iter().collect();

        Ok((input, map))
    }
}

impl<'input> Extract<'input> for RawDict<'input> {
    fn extract(input: &'input [u8]) -> IResult<&'input [u8], Self> {
        let (input, map) = HashMap::<String, &'input [u8]>::extract(input)?;
        Ok((input, Self(map)))
    }
}

impl<'input, T> Extract<'input> for Dictionary<T>
where
    T: Extract<'input>,
{
    fn extract(input: &'input [u8]) -> IResult<&'input [u8], Self> {
        let (input, map) = HashMap::<String, &'input [u8]>::extract(input)?;
        let map: Result<HashMap<String, T>> = map
            .into_iter()
            .map(|(key, val)| val.parse::<T>().map(|v| (key, v)))
            .collect();
        let map = map.map_err(|_| {
            nom::Err::Error(nom::error::Error::from_error_kind(
                input,
                nom::error::ErrorKind::IsNot,
            ))
        })?;
        Ok((input, Self(map)))
    }
}

impl<T> Deref for Dictionary<T> {
    type Target = HashMap<String, T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Dictionary<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'input> RawDict<'input> {
    pub fn pop<T>(&mut self, key: &str) -> Result<T>
    where
        T: Extract<'input>,
    {
        let result = self
            .remove(key)
            .ok_or_else(|| error::ExtractionError::KeyNotFound(key.into()))?
            .parse()?;

        Ok(result)
    }

    pub fn pop_opt<T>(&mut self, key: &str) -> Result<Option<T>>
    where
        T: Extract<'input>,
    {
        let result = self.remove(key).map(|obj| obj.parse()).transpose()?;
        Ok(result)
    }

    pub fn convert<T>(self) -> Result<HashMap<String, T>>
    where
        T: Extract<'input>,
    {
        self.0
            .into_iter()
            .map(|(key, value)| value.parse::<T>().map(|r| (key, r)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(b"<</Key (value)>>", vec![("Key", b"(value)".as_slice())])]
    #[case(b"<</Test true/Ok false>>", vec![("Test", b"true".as_slice()), ("Ok", b"false".as_slice())])]
    fn dictionary(#[case] input: &[u8], #[case] result: Vec<(&str, &[u8])>) {
        let dict = result
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();

        let (_, Dictionary(r)) = RawDict::extract(input).unwrap();

        assert_eq!(r, dict);
    }

    #[rstest]
    #[case(b"<</Key (value)>>", "Key", "value")]
    #[case(b"<</Key (test)/Ok (false)>>", "Key", "test")]
    #[case(b"<</Key (test)/Ok (false)>>", "Ok", "false")]
    fn dictionary_pop(#[case] input: &[u8], #[case] key: &str, #[case] expected: &str) {
        let (_, mut dict) = RawDict::extract(input).unwrap();
        let result: String = dict.pop(key).unwrap();
        assert_eq!(result, expected);
    }

    #[rstest]
    #[case(b"<</Key (value)>>", "Key", Some("value"))]
    #[case(b"<</Key (test)/Ok (false)>>", "Key", Some("test"))]
    #[case(b"<</Key (test)/Ok (false)>>", "NotHere", None)]
    fn dictionary_pop_opt(#[case] input: &[u8], #[case] key: &str, #[case] expected: Option<&str>) {
        let (_, mut dict) = RawDict::extract(input).unwrap();
        let result: Option<String> = dict.pop_opt(key).unwrap();
        let expected = expected.map(|s| s.to_string());
        assert_eq!(result, expected);
    }
}
