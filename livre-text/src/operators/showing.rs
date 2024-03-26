use livre_extraction::{extract, Extract};
use livre_utilities::take_whitespace1;
use nom::{branch::alt, bytes::complete::tag, combinator::map, sequence::tuple, IResult};

#[derive(Debug, PartialEq, Clone)]
pub struct ShowTj(pub String);

impl Extract<'_> for ShowTj {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        let (input, text) = extract_text(input)?;
        let (input, _) = tuple((take_whitespace1, tag(b"Tj")))(input)?;
        Ok((input, Self(text)))
    }
}

fn extract_text(input: &[u8]) -> IResult<&[u8], String> {
    alt((String::extract, map(char::extract, |c| c.into())))(input)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(b"(test) Tj", "test")]
    #[case(b"<0048> Tj", "H")]
    #[case(b"<0057> Tj", "W")]
    #[case(b"<0052> Tj", "R")]
    fn show_tj(#[case] input: &[u8], #[case] expected: &str) {
        let (_, ShowTj(text)) = extract(input).unwrap();
        assert_eq!(text, expected)
    }
}
