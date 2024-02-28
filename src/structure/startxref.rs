use nom::{
    bytes::complete::{tag, take_until},
    IResult,
};

use crate::utilities::{parse_digits, take_whitespace1};

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct StartXRef(pub u64);

const STARTXREF_TAG: &[u8] = b"startxref";

impl StartXRef {
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, _) = tag("startxref")(input)?;
        let (input, _) = take_whitespace1(input)?;
        let (input, startxref) = parse_digits(input)?;
        let (input, _) = take_whitespace1(input)?;
        let (input, _) = tag("%%EOF")(input)?;

        Ok((input, Self(startxref)))
    }

    pub fn search(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, _) = take_until(STARTXREF_TAG)(input)?;
        Self::parse(input)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use indoc::indoc;
    use rstest::rstest;

    fn parse(input: &[u8]) -> StartXRef {
        let (_, startxref) = StartXRef::parse(input).unwrap();
        startxref
    }

    fn search(input: &[u8]) -> StartXRef {
        let (_, startxref) = StartXRef::search(input).unwrap();
        startxref
    }

    #[rstest]
    #[case(b"startxref\n18788\n%%EOF", 18788)]
    fn parse_startxref(#[case] input: &[u8], #[case] startxref: u64) {
        let mut new_input = indoc! {b"
            xref
            0 1
            0000000000 65535 f 
            3 1
            0000025325 00000 n 
            23 2
            0000025518 00002 n 
            0000025635 00000 n 
            30 1
            0000025777 00000 n \r
        "}
        .to_vec();
        new_input.extend(input);

        assert_eq!(parse(input), StartXRef(startxref));
        assert_eq!(search(input), StartXRef(startxref));
        assert_eq!(search(&new_input), StartXRef(startxref));
    }
}
