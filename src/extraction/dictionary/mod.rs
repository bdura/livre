use nom::{
    error::{Error, ErrorKind, ParseError},
    multi::many0,
    Err, IResult,
};
use std::collections::HashMap;

use crate::utilities::{take_whitespace, take_within_balanced};

use super::Extract;

mod utilities;
use utilities::parse_key_value;

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Dictionary<'input>(pub HashMap<String, &'input [u8]>);

impl<'input> Extract<'input> for Dictionary<'input> {
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

        Ok((input, Self(map)))
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

        let (_, Dictionary(r)) = Dictionary::extract(input).unwrap();

        assert_eq!(r, dict);
    }
}
