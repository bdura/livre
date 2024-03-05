use livre_extraction::{Extract, Reference};
use livre_utilities::take_whitespace1;
use nom::{bytes::complete::tag, IResult};

#[derive(Debug, Clone, PartialEq, Extract)]
pub struct TrailerDict {
    pub size: usize,
    pub prev: Option<usize>,
    pub root: Reference,
    // pub encrypt: Encrypt,
    pub info: Reference,
    // #[livre(rename = "ID")]
    // pub id: MaybeArray<HexString>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Trailer(pub TrailerDict);

impl Extract<'_> for Trailer {
    fn extract(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, _) = tag(b"trailer")(input)?;
        let (input, _) = take_whitespace1(input)?;
        let (input, dict) = TrailerDict::extract(input)?;
        let (input, _) = take_whitespace1(input)?;

        Ok((input, Self(dict)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

        let (_, Trailer(trailer)) = Trailer::extract(input).unwrap();

        assert_eq!(
            trailer,
            TrailerDict {
                size: 22,
                prev: None,
                root: Reference::new(2, 0),
                info: Reference::new(1, 0)
            }
        )
    }
}
