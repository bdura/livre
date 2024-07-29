use crate::parsers::take_whitespace1;
use crate::parsers::Extract;
use crate::serde::extract_deserialize;
use nom::{bytes::complete::tag, IResult};

use super::super::TrailerDict;

#[derive(Debug, PartialEq, Clone)]
pub struct PlainTrailer(pub TrailerDict);

impl Extract<'_> for PlainTrailer {
    fn extract(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, _) = tag(b"trailer")(input)?;
        let (input, _) = take_whitespace1(input)?;
        let (input, dict) = extract_deserialize(input)?;
        let (input, _) = take_whitespace1(input)?;

        Ok((input, Self(dict)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::{extract, TypedReference};
    use indoc::indoc;

    #[test]
    fn parse_trailer() {
        let input = indoc! {b"
            trailer
            <</Size 22
            /Root 2 0 R
            /Info 1 0 R
            /ID [<0011>
            <001>]>>
        "};

        let (_, PlainTrailer(trailer)) = extract(input).unwrap();

        assert_eq!(
            trailer,
            TrailerDict {
                size: 22,
                prev: None,
                root: TypedReference::new(2, 0),
                info: TypedReference::new(1, 0)
            }
        )
    }
}
