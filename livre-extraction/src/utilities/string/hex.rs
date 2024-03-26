use nom::{multi::many0, IResult};

use livre_utilities::parse_hexadecimal_bigram;

use crate::{extract, Angles, Extract};

#[derive(Debug, PartialEq, Clone)]
pub struct HexBytes(pub Vec<u8>);

impl Extract<'_> for HexBytes {
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
        let (_, HexBytes(bytes)) = HexBytes::extract(input).unwrap();
        assert_eq!(bytes, result);
    }
}
