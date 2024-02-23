use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{digit0, digit1},
    combinator::{opt, recognize},
    sequence::{pair, separated_pair},
    IResult,
};

use crate::utilities::parse_sign;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Real(pub f32);

impl Real {
    fn parse_unsigned(input: &[u8]) -> IResult<&[u8], &[u8]> {
        alt((
            recognize(separated_pair(digit1, tag(b"."), digit0)),
            recognize(separated_pair(digit0, tag(b"."), digit1)),
        ))(input)
    }

    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, num) = recognize(pair(opt(parse_sign), Self::parse_unsigned))(input)?;

        // SAFETY: we know for a fact that `num` only includes ascii characters
        let num_str = unsafe { std::str::from_utf8_unchecked(num) };

        let num = num_str
            .parse()
            .expect("[+-]?\\d*.\\d* is parseable as an integer.");

        Ok((input, Self(num)))
    }
}

impl From<Real> for f32 {
    fn from(value: Real) -> Self {
        value.0
    }
}

impl From<f32> for Real {
    fn from(value: f32) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn parse(input: &[u8]) -> Real {
        let (_, obj) = Real::parse(input).unwrap();
        obj
    }

    #[allow(clippy::bool_assert_comparison)]
    #[rstest]
    #[case(b"1.0", 1.)]
    #[case(b".0", 0.)]
    #[case(b"-0.0", 0.)]
    #[case(b"-10.0", -10.)]
    fn test_parse(#[case] input: &[u8], #[case] result: f32) {
        assert_eq!(parse(input), result.into());
        assert_eq!(result, parse(input).into());
    }
}
