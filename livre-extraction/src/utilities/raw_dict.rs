use nom::IResult;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use crate::error::{self, Result};

use crate::extraction::{Extract, Parse};

use super::RawValue;

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct RawDict<'input>(pub HashMap<String, &'input [u8]>);

impl<'input> Extract<'input> for RawDict<'input> {
    fn extract(input: &'input [u8]) -> IResult<&'input [u8], Self> {
        let (input, map) = HashMap::<String, RawValue>::extract(input)?;

        let map = map
            .into_iter()
            .map(|(key, RawValue(value))| (key, value))
            .collect();

        Ok((input, Self(map)))
    }
}

impl<'input> Deref for RawDict<'input> {
    type Target = HashMap<String, &'input [u8]>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'input> DerefMut for RawDict<'input> {
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
    fn raw_dict(#[case] input: &[u8], #[case] result: Vec<(&str, &[u8])>) {
        let dict = result
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();

        let (_, RawDict(r)) = RawDict::extract(input).unwrap();

        assert_eq!(r, dict);
    }

    #[rstest]
    #[case(b"<</Key (value)>>", "Key", "value")]
    #[case(b"<</Key (test)/Ok (false)>>", "Key", "test")]
    #[case(b"<</Key (test)/Ok (false)>>", "Ok", "false")]
    fn raw_dict_pop(#[case] input: &[u8], #[case] key: &str, #[case] expected: &str) {
        let (_, mut dict) = RawDict::extract(input).unwrap();
        let result: String = dict.pop(key).unwrap();
        assert_eq!(result, expected);
    }

    #[rstest]
    #[case(b"<</Key (value)>>", "Key", Some("value"))]
    #[case(b"<</Key (test)/Ok (false)>>", "Key", Some("test"))]
    #[case(b"<</Key (test)/Ok (false)>>", "NotHere", None)]
    fn raw_dict_pop_opt(#[case] input: &[u8], #[case] key: &str, #[case] expected: Option<&str>) {
        let (_, mut dict) = RawDict::extract(input).unwrap();
        let result: Option<String> = dict.pop_opt(key).unwrap();
        let expected = expected.map(|s| s.to_string());
        assert_eq!(result, expected);
    }

    #[test]
    fn raw_dict_convert() {
        let input = b"<</Key1 (test)/Key2 (false)>>";
        let (_, raw) = RawDict::extract(input).unwrap();
        let string_map = raw.convert::<String>().unwrap();

        assert_eq!(string_map.len(), 2);

        assert_eq!(string_map.get("Key1"), Some(&"test".to_string()));
        assert_eq!(string_map.get("Key2"), Some(&"false".to_string()));
    }
}
