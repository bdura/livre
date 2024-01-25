use std::collections::HashMap;

use nom::{bytes::complete::tag, IResult};

use super::{
    utilities::{parse_digits, take_whitespace},
    Object,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Trailer {
    dict: HashMap<String, Object>,
    start_xref: usize,
}

impl Trailer {
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, _) = tag(b"trailer")(input)?;
        let (input, _) = take_whitespace(input)?;
        let (input, obj) = Object::parse(input)?;

        let dict = if let Object::Dictionary(dict) = obj {
            dict
        } else {
            panic!("The parsed object should be a dict, per the PDF specs.");
        };

        let (input, _) = take_whitespace(input)?;
        let (input, _) = tag(b"startxref")(input)?;
        let (input, _) = take_whitespace(input)?;
        let (input, start_xref) = parse_digits(input)?;

        Ok((input, Trailer { dict, start_xref }))
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
        "};

        let (_, trailer) = Trailer::parse(input).unwrap();
        assert_eq!(trailer.start_xref, 18799);
    }
}
