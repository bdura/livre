use nom::{multi::many0, IResult};

use crate::parsers::parse_hexadecimal_bigram;
use serde::{de::Visitor, Deserialize};

use crate::parsers::extraction::{extract, Angles, Extract};

#[derive(Debug, PartialEq, Clone)]
pub struct HexBytes(pub Vec<u8>);

struct HexBytesVisitor;

impl<'de> Visitor<'de> for HexBytesVisitor {
    type Value = HexBytes;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "PDF hexbytes")
    }

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(HexBytes(v))
    }
}

impl<'de> Deserialize<'de> for HexBytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_byte_buf(HexBytesVisitor)
    }
}

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
