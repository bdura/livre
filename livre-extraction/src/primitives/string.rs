use crate::{encoding::decode_str, extract, extraction::Extract, LitBytes};

impl Extract<'_> for String {
    fn extract(input: &[u8]) -> nom::IResult<&[u8], Self> {
        let (input, LitBytes(bytes)) = extract(input)?;
        let res = decode_str(&bytes);
        Ok((input, res.to_string()))
    }
}

#[cfg(test)]
mod tests {

    use rstest::rstest;

    use super::*;
    use crate::parse;

    #[rstest]
    #[case(b"(abcd)", "abcd")]
    #[case(b"(test)", "test")]
    #[case(b"(test\n)", "test\n")]
    #[case(b"(test (with inner parenthesis))", "test (with inner parenthesis)")]
    #[case(b"(\\0533)", "+3")]
    #[case(b"(te\\\\st)", "te\\st")]
    #[case(b"(te\\\nst)", "test")]
    fn string(#[case] input: &[u8], #[case] expected: &str) {
        let (_, parsed) = String::extract(input).unwrap();
        assert_eq!(parsed, expected);
        assert_eq!(parse::<String>(input).unwrap(), expected);
    }
}
