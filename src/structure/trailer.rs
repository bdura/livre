use nom::{bytes::complete::tag, IResult};

use crate::{
    objects::{Dictionary, Reference},
    utilities::{parse_digits, take_whitespace1},
};

#[derive(Debug, Clone, PartialEq)]
pub struct Trailer {
    pub size: usize,
    pub prev: Option<usize>,
    // TODO: add other fields...
    pub root: Reference,
    //pub encrypt
    //pub info
    //pub id
    /// Last but not least
    pub startxref: usize,
}

impl Trailer {
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, _) = tag(b"trailer")(input)?;
        let (input, _) = take_whitespace1(input)?;
        let (input, mut dict) = Dictionary::parse(input)?;
        let (input, _) = take_whitespace1(input)?;
        let (input, _) = tag(b"startxref")(input)?;
        let (input, _) = take_whitespace1(input)?;
        let (input, startxref) = parse_digits(input)?;
        let (input, _) = take_whitespace1(input)?;
        let (input, _) = tag(b"%%EOF")(input)?;

        let size = dict
            .pop("Size")
            .expect("Size is a mandatory field compatible with usize");
        let root = dict.pop("Root").expect("Root must exist");
        let prev = dict.pop_opt("Prev").expect("Prev is an integer");

        Ok((
            input,
            Self {
                size,
                prev,
                root,
                startxref,
            },
        ))
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
                /ID [<81b14aafa313db63dbd6f981e49f94f4>
                <81b14aafa313db63dbd6f981e49f94f4>
                ] >>
            startxref
            18799
            %%EOF
        "};

        let (_, trailer) = Trailer::parse(input).unwrap();
        assert_eq!(
            trailer,
            Trailer {
                startxref: 18799,
                size: 22,
                prev: None,
                root: Reference {
                    object: 2,
                    generation: 0
                }
            }
        )
    }
}
