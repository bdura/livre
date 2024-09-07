use winnow::{
    ascii::hex_uint,
    combinator::{delimited, repeat, trace},
    error::ContextError,
    stream::AsChar,
    token::take_while,
    BStr, PResult, Parser,
};

use crate::Extract;

/// A PDF Hexadecimal String.
///
/// Quoting the specs:
///
/// > Strings may also be written in hexadecimal form, which is useful for including
/// > arbitrary binary data in a PDF file.
/// > A hexadecimal string shall be written as a sequence of hexadecimal digits
/// > encoded as ASCII characters and enclosed within angle brackets.
pub struct HexadecimalString(pub Vec<u8>);

impl Extract<'_> for HexadecimalString {
    fn extract(input: &mut &BStr) -> PResult<Self> {
        trace(
            "livre-hexadecimal-string",
            delimited(b'<', repeat(1.., parse_hexadecimal_bigram), b'>').map(Self),
        )
        .parse_next(input)
    }
}

/// Parse up to two bytes to get the number represented by the hexadecimal code.
///
/// The PDF specs mention the case of odd-number characters:
///
/// > If the final digit of a hexadecimal string is missing — that is, if there
/// > is an odd number of digits — the final digit shall be assumed to be 0.
pub fn parse_hexadecimal_bigram(input: &mut &BStr) -> PResult<u8> {
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
}
