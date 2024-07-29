use nom::{multi::many0, IResult};

use crate::parsers::parse_hexadecimal_bigram;

use crate::parsers::{extract, Angles, Extract};

#[derive(Debug, PartialEq, Clone)]
pub struct HexString(pub Vec<u8>);

impl Extract<'_> for HexString {
    fn extract(input: &'_ [u8]) -> IResult<&'_ [u8], Self> {
        let (input, Angles(value)) = extract(input)?;
        let (d, uvec) = many0(parse_hexadecimal_bigram)(value)?;
        assert!(d.is_empty());
        Ok((input, Self(uvec)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(b"<901FA3>", &[144, 31, 163])]
    #[case(b"<901FA>", &[144, 31, 160])]
    fn hex_string(#[case] input: &[u8], #[case] result: &[u8]) {
        let (_, HexString(bytes)) = HexString::extract(input).unwrap();
        assert_eq!(bytes, result);
    }
}
