use crate::{encoding::pdf_decode, extract, extraction::Extract, LitBytes};

static UTF8_MARKER: &[u8] = &[239, 187, 191];
static UTF16BE_MARKER: &[u8] = &[254, 255];

impl Extract<'_> for String {
    fn extract(input: &[u8]) -> nom::IResult<&[u8], Self> {
        let (input, LitBytes(bytes)) = extract(input)?;

        let res = if bytes.starts_with(UTF8_MARKER) {
            std::str::from_utf8(&bytes[3..])
                .expect("Per the specs, the string is UTF-8 encoded")
                .to_owned()
        } else if bytes.starts_with(UTF16BE_MARKER) {
            let utf16: Vec<u16> = bytes[2..]
                .chunks_exact(2)
                .map(|b| u16::from_be_bytes([b[0], b[1]]))
                .collect();
            String::from_utf16(&utf16).expect("Per the specs, the string is UTF-16BE encoded")
        } else {
            let bytes = pdf_decode(&bytes);
            std::str::from_utf8(&bytes)
                .expect("Per the specs, the string is UTF-8 encoded")
                .to_owned()
        };

        Ok((input, res))
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
