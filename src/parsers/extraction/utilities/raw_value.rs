use nom::{
    branch::alt,
    bytes::complete::take_till,
    combinator::{recognize, verify},
    IResult,
};

use super::{Angles, Brackets, Extract, Name, Parentheses};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RawValue<'input>(pub &'input [u8]);

impl<'input> Extract<'input> for RawValue<'input> {
    fn extract(input: &'input [u8]) -> IResult<&'input [u8], Self> {
        let (input, value) = alt((
            recognize(Brackets::extract),
            recognize(Angles::extract),
            recognize(Parentheses::extract),
            verify(take_till(|b| b == b'/'), |v: &[u8]| !v.is_empty()),
            recognize(Name::extract),
        ))(input)?;
        Ok((input, Self(value)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::rstest;

    #[rstest]
    #[case(b"(test)", b"(test)")]
    #[case(b"true ", b"true ")]
    #[case(b"true/", b"true")]
    #[case(b"/Value", b"/Value")]
    #[case(
        b"[<81b14aafa313db63dbd6f981e49f94f4>] ",
        b"[<81b14aafa313db63dbd6f981e49f94f4>]"
    )]
    #[case(b"<</Test true>>", b"<</Test true>>")]
    fn raw_value(#[case] input: &[u8], #[case] expected: &[u8]) {
        let (_, RawValue(v)) = RawValue::extract(input).unwrap();
        assert_eq!(v, expected);
    }
}
