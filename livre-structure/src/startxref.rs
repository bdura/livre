use livre_extraction::{extract, Extract};
use livre_utilities::take_whitespace1;
use nom::{
    bytes::complete::{tag, take_until},
    IResult,
};

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct StartXRef(pub usize);

const STARTXREF_TAG: &[u8] = b"startxref";

impl Extract<'_> for StartXRef {
    fn extract(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, _) = tag(STARTXREF_TAG)(input)?;
        let (input, _) = take_whitespace1(input)?;
        let (input, startxref) = extract(input)?;
        let (input, _) = take_whitespace1(input)?;

        Ok((input, Self(startxref)))
    }
}

impl StartXRef {
    pub fn find(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, _) = take_until(STARTXREF_TAG)(input)?;
        Self::extract(input)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use indoc::indoc;
    use rstest::rstest;

    #[rstest]
    #[case(
        indoc! {b"
            startxref
            18788
            %%EOF
        "},
        18788
    )]
    #[case(
        indoc! {b"
            startxref
            10
            %%EOF
        "},
        10
    )]
    fn startxref(#[case] input: &[u8], #[case] expected: usize) {
        let (_, StartXRef(startxref)) = StartXRef::extract(input).unwrap();
        assert_eq!(startxref, expected);

        let mut prepended = b"1 or 2. (random)/words".to_vec();
        prepended.extend(input);

        let (_, StartXRef(startxref)) = StartXRef::find(&prepended).unwrap();
        assert_eq!(startxref, expected);
    }
}
