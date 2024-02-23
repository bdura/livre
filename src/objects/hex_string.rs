use nom::{branch::alt, bytes::complete::take, combinator::map, multi::many0, IResult};

use crate::utilities::take_within_balanced;

/// Represents a boolean within a PDF.
#[derive(Debug, PartialEq, Clone)]
pub struct HexString(pub(crate) Vec<u8>);

impl HexString {
    fn parse_hexadecimal_bigram(input: &[u8]) -> IResult<&[u8], u8> {
        fn inner(input: &[u8]) -> u8 {
            let len = input.len();

            let mut res = {
                // SAFETY: we know for a fact that the supplied input
                // will hold that invariant.
                let num = unsafe { std::str::from_utf8_unchecked(input) };
                u8::from_str_radix(num, 16).unwrap()
            };

            if len == 1 {
                res *= 16;
            }

            res
        }

        alt((map(take(2usize), inner), map(take(1usize), inner)))(input)
    }

    pub(crate) fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, value) = take_within_balanced(b'<', b'>')(input)?;
        let (d, uvec) = many0(Self::parse_hexadecimal_bigram)(value)?;
        assert!(d.is_empty());
        Ok((input, Self(uvec)))
    }
}

impl From<HexString> for Vec<u8> {
    fn from(value: HexString) -> Self {
        value.0
    }
}
impl From<Vec<u8>> for HexString {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}
impl From<&[u8]> for HexString {
    fn from(value: &[u8]) -> Self {
        Self(value.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn parse(input: &[u8]) -> HexString {
        let (_, obj) = HexString::parse(input).unwrap();
        obj
    }

    #[allow(clippy::bool_assert_comparison)]
    #[rstest]
    #[case(b"<901FA3>", &[144, 31, 163])]
    #[case(b"<901FA>", &[144, 31, 160])]
    fn test_parse(#[case] input: &[u8], #[case] result: &[u8]) {
        assert_eq!(parse(input), result.into());
        let result = result.to_owned();
        let parsed: Vec<u8> = parse(input).into();
        assert_eq!(parsed, result);
    }
}
