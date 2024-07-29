use nom::{
    error::{Error, ParseError},
    Err,
};

use super::{extract, Extract, HexBytes};

impl Extract<'_> for char {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        let (input, HexBytes(bytes)) = extract(input)?;
        if bytes.is_empty() || bytes.len() > 4 {
            return Err(Err::Error(Error::from_error_kind(
                input,
                nom::error::ErrorKind::Count,
            )));
        }

        let mut res: u32 = 0;

        for (i, b) in bytes.into_iter().rev().enumerate() {
            res += 256u32.pow(i as u32) * (b as u32)
        }

        let c = char::from_u32(res).unwrap();

        Ok((input, c))
    }
}
#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(b"<0048>", 'H')]
    #[case(b"<0057>", 'W')]
    #[case(b"<0052>", 'R')]
    fn extract_char(#[case] input: &[u8], #[case] expected: char) {
        let (_, c) = char::extract(input).unwrap();
        assert_eq!(c, expected)
    }
}
