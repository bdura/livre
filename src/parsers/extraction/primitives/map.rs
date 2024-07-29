use std::collections::HashMap;

use super::take_whitespace;
use nom::{multi::many0, IResult};

use super::{extract, DoubleAngles, Extract, Name};

pub type Map<T> = HashMap<String, T>;

impl<'input, T> Extract<'input> for Map<T>
where
    T: Extract<'input>,
{
    fn extract(input: &'input [u8]) -> IResult<&'input [u8], Self> {
        let (input, DoubleAngles(value)) = extract(input)?;
        let (value, _) = take_whitespace(value)?;

        let (r, array) = many0(parse_key_value)(value)?;

        assert!(
            r.is_empty(),
            "Remainder is not empty: {:?}",
            String::from_utf8_lossy(r)
        );

        let map: Self = array.into_iter().collect();

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
    let (input, _) = take_whitespace(input)?;

    Ok((input, (key, value)))
}

#[cfg(test)]
mod tests {

    use crate::parsers::extraction::RawValue;

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

    #[rstest]
    #[case(b"<</Val 1 /Test -12>>", vec![("Val".to_string(), 1), ("Test".to_string(), -12)])]
    #[case(b"<< /Val 1 >>", vec![("Val".to_string(), 1)])]
    fn map_i32(#[case] input: &[u8], #[case] expected: Vec<(String, i32)>) {
        let dict: Map<i32> = expected.into_iter().collect();
        let (_, map) = Map::<i32>::extract(input).unwrap();
        assert_eq!(map, dict);
    }
}
