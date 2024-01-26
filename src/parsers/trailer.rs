use std::collections::HashMap;

use nom::{
    bytes::complete::tag,
    error::{Error, ErrorKind, ParseError},
    Err, IResult,
};

use super::{utilities::take_whitespace, Object};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Trailer(HashMap<String, Object>);

impl Trailer {
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, _) = tag(b"trailer")(input)?;
        let (input, _) = take_whitespace(input)?;
        let (input, obj) = Object::parse(input)?;

        if let Object::Dictionary(dict) = obj {
            Ok((input, Trailer(dict)))
        } else {
            Err(Err::Error(Error::from_error_kind(input, ErrorKind::Fail)))
        }
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

        let (_, _trailer) = Trailer::parse(input).unwrap();
    }
}
