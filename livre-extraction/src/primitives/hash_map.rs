use std::collections::HashMap;

use livre_utilities::{parse_dict_body, take_whitespace};
use nom::{multi::many0, IResult};

use crate::{Extract, Name};

impl<'input, T> Extract<'input> for HashMap<String, T>
where
    T: Extract<'input>,
{
    fn extract(input: &'input [u8]) -> IResult<&'input [u8], Self> {
        let (input, value) = parse_dict_body(input)?;

        let (value, _) = take_whitespace(value)?;
        let (r, array) = many0(parse_key_value)(value)?;

        let (r, _) = take_whitespace(r)?;

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

    let (input, value) = T::extract(input)?;

    Ok((input, (key, value)))
}

#[cfg(test)]
mod tests {
    use crate::utilities::RawValue;

    use super::*;

    use rstest::rstest;

    #[rstest]
    #[case(b"/Name1 (test)", "Name1", b"(test)")]
    #[case(b"/Bool true ", "Bool", b"true ")]
    #[case(b"/Bool true/", "Bool", b"true")]
    #[case(b"/NamedValue /Value", "NamedValue", b"/Value")]
    #[case(b"/Dict <</Test true>>", "Dict", b"<</Test true>>")]
    #[case(
        b"/ID [<81b14aafa313db63dbd6f981e49f94f4>] ",
        "ID",
        b"[<81b14aafa313db63dbd6f981e49f94f4>]"
    )]
    fn key_value_bytes(#[case] input: &[u8], #[case] key: &str, #[case] value: &[u8]) {
        let (_, (k, RawValue(v))) = parse_key_value::<RawValue>(input).unwrap();
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
