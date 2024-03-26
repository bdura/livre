use std::borrow::Cow;

use crate::{encoding::decode_str, extract, extraction::Extract, LitBytes};

impl<'input> Extract<'input> for Cow<'input, str> {
    fn extract(input: &'input [u8]) -> nom::IResult<&'input [u8], Self> {
        let (input, LitBytes(bytes)) = extract(input)?;

        let res = match bytes {
            Cow::Borrowed(input) => decode_str(input),
            Cow::Owned(input) => Cow::Owned(decode_str(&input).to_string()),
        };
        Ok((input, res))
    }
}

impl Extract<'_> for String {
    fn extract(input: &[u8]) -> nom::IResult<&[u8], Self> {
        let (input, cow) = Cow::<'_, str>::extract(input)?;
        Ok((input, cow.to_string()))
    }
}

#[cfg(test)]
mod tests {

    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(b"(abcd)", "abcd")]
    #[case(b"(test)", "test")]
    #[case(b"(test\n)", "test\n")]
    #[case(b"(test (with inner parenthesis))", "test (with inner parenthesis)")]
    #[case(b"(\\0533)", "+3")]
    #[case(b"(te\\\\st)", "te\\st")]
    #[case(b"(te\\\nst)", "test")]
    fn pdf_encoding(#[case] input: &[u8], #[case] expected: &str) {
        let (_, parsed) = String::extract(input).unwrap();
        assert_eq!(parsed, expected);
    }
}
