use std::collections::HashMap;

use livre_utilities::{take_whitespace, take_within_balanced};
use nom::{
    bytes::complete::take_till,
    error::{Error, ErrorKind, ParseError},
    multi::many0,
    Err, IResult,
};

use crate::{Extract, Name, Parse};

impl<'input, T> Extract<'input> for HashMap<String, T>
where
    T: Extract<'input>,
{
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

        let map: HashMap<String, T> = array.into_iter().collect();

        Ok((input, map))
    }
}

fn parse_key_value<'input, T>(input: &'input [u8]) -> IResult<&'input [u8], (String, T)>
where
    T: Extract<'input>,
{
    let (input, Name(key)) = Name::extract(input)?;
    let (input, _) = take_whitespace(input)?;
    let (input, value) = take_till(|b| b == b'/')(input)?;

    // FIXME: handle error.
    let parsed = value.parse().unwrap();

    Ok((input, (key, parsed)))
}

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::rstest;

    #[rstest]
    #[case(b"/Name1 (test)", "Name1", b"(test)")]
    #[case(b"/Bool true ", "Bool", b"true ")]
    #[case(b"/Bool true/", "Bool", b"true")]
    fn key_value_bytes(#[case] input: &[u8], #[case] key: &str, #[case] value: &[u8]) {
        let (_, (k, v)) = parse_key_value::<&[u8]>(input).unwrap();
        assert_eq!(k, key);
        assert_eq!(v, value);
    }

    #[rstest]
    #[case(b"/Val 1 ", "Val", 1)]
    #[case(b"/Test -34/", "Test", -34)]
    fn key_value_i32(#[case] input: &[u8], #[case] key: &str, #[case] value: i32) {
        let (_, (k, v)) = parse_key_value::<i32>(input).unwrap();
        assert_eq!(k, key);
        assert_eq!(v, value);
    }
}
