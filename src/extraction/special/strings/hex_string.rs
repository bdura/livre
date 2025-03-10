use std::fmt::Debug;

use winnow::{
    ascii::hex_uint,
    combinator::{delimited, repeat, trace},
    error::ContextError,
    stream::AsChar,
    token::take_while,
    BStr, PResult, Parser,
};

use crate::extraction::Extract;

/// A PDF Hexadecimal String.
///
/// Note that although they are named `Strings` in the PDF specification, [`HexadecimalString`]s
/// and [`LiteralString`](super::LiteralString)s are not necessarily valid UTF-8 and, as such,
/// not represetable by a Rust [`String`].
///
/// Quoting the specs:
///
/// > Strings may also be written in hexadecimal form, which is useful for including
/// > arbitrary binary data in a PDF file.
/// > A hexadecimal string shall be written as a sequence of hexadecimal digits
/// > encoded as ASCII characters and enclosed within angle brackets.
#[derive(PartialEq, Eq, Clone)]
pub struct HexadecimalString(pub Vec<u8>);

impl Debug for HexadecimalString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HexadecimalString<")?;

        for b in &self.0 {
            write!(f, "{b:02X}")?;
        }

        write!(f, ">")?;

        Ok(())
    }
}

impl Extract<'_> for HexadecimalString {
    fn extract(input: &mut &BStr) -> PResult<Self> {
        trace(
            "livre-hexadecimal-string",
            delimited(b'<', repeat(1.., parse_hexadecimal_bigram), b'>').map(Self),
        )
        .parse_next(input)
    }
}

impl<T> From<T> for HexadecimalString
where
    T: Into<Vec<u8>>,
{
    fn from(value: T) -> Self {
        let vec = value.into();
        Self(vec)
    }
}

/// Parse up to two bytes to get the number represented by the hexadecimal code.
///
/// The PDF specs mention the case of odd-number characters:
///
/// > If the final digit of a hexadecimal string is missing — that is, if there
/// > is an odd number of digits — the final digit shall be assumed to be 0.
fn parse_hexadecimal_bigram(input: &mut &BStr) -> PResult<u8> {
    trace("livre-hex-bigram", move |i: &mut &BStr| {
        let num = take_while(1..=2, |b: u8| b.is_hex_digit()).parse_next(i)?;

        let len = num.len();

        let mut n = hex_uint::<_, _, ContextError>
            .parse(num)
            .expect("correct by construction");

        // Final digit is assumed to be 0.
        if len == 1 {
            n *= 16;
        }

        Ok(n)
    })
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use crate::extraction::extract;

    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(b"<901FA3>", &[144, 31, 163])]
    #[case(b"<901FA>", &[144, 31, 160])]
    fn hex_string(#[case] input: &[u8], #[case] expected: &[u8]) {
        let HexadecimalString(bytes) = extract(&mut input.as_ref()).unwrap();
        assert_eq!(bytes, expected);
    }

    #[rstest]
    #[case(&[0x90, 0x1F, 0xA3], "901FA3")]
    #[case(&[0x01, 0x0, 0x0F], "01000F")]
    fn hex_string_debug(#[case] input: &[u8], #[case] expected: &str) {
        let expected = format!("HexadecimalString<{expected}>");
        let result = HexadecimalString::from(input);
        assert_eq!(format!("{result:?}"), expected);
    }
}
