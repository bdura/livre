use livre_extraction::{extract, Extract};
use livre_utilities::take_whitespace1;
use nom::{bytes::complete::tag, sequence::tuple};

#[derive(Debug, PartialEq, Clone)]
pub struct ShowTj(pub String);

impl Extract<'_> for ShowTj {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        let (input, text) = extract(input)?;
        let (input, _) = tuple((take_whitespace1, tag(b"Tj")))(input)?;
        Ok((input, Self(text)))
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(b"(test) Tj", "test")]
    fn show_tj(#[case] input: &[u8], #[case] expected: &str) {
        let (_, ShowTj(text)) = extract(input).unwrap();
        assert_eq!(text, expected)
    }
}
