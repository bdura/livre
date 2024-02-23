use nom::{bytes::complete::tag, IResult};

use crate::{
    objects::Dictionary,
    utilities::{parse_digits, take_whitespace1},
};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Trailer {
    pub startxref: usize,
    pub size: usize,
    // TODO: add other fields...
    //pub prev
    //pub root
    //pub encrypt
    //pub info
    //pub id
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
            .remove("Size")
            .expect("Size is a mandatory field")
            .try_into()
            .expect("Size must use an integer type coerceable to usize");

        Ok((input, Self { size, startxref }))
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
                size: 22
            }
        )
    }
}
