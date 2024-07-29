use crate::objects::{Indirect, Reference};
use crate::parsers::take_whitespace;
use crate::parsers::{extract, Extract};
use nom::{branch::alt, sequence::separated_pair, IResult};

use super::super::TrailerDict;
use super::{PlainCrossRefs, RefLocation, XRefStream};

use super::PlainTrailer;

pub type XRefVec = Vec<(Reference, RefLocation)>;

#[derive(Debug, PartialEq, Clone)]
pub struct Trailer {
    pub dict: TrailerDict,
    pub refs: XRefVec,
}

impl Extract<'_> for Trailer {
    fn extract(input: &[u8]) -> IResult<&[u8], Self> {
        alt((parse_plain, parse_stream))(input)
    }
}

pub fn parse_plain(input: &[u8]) -> IResult<&[u8], Trailer> {
    let (input, (PlainCrossRefs(refs), PlainTrailer(dict))) =
        separated_pair(extract, take_whitespace, extract)(input)?;

    let trailer = Trailer { dict, refs };

    Ok((input, trailer))
}

pub fn parse_stream(input: &[u8]) -> IResult<&[u8], Trailer> {
    let (input, Indirect { inner, .. }) = extract(input)?;

    let XRefStream { refs, dict } = inner;
    let trailer = Trailer { dict, refs };

    Ok((input, trailer))
}

#[cfg(test)]
mod tests {
    use crate::objects::Bytes;
    use crate::parsers::TypedReference;
    use indoc::indoc;

    use super::*;

    #[test]
    fn plain_trailer() {
        let input = indoc! {b"
            xref
            0 0
            trailer
            <</Size 194/Root 1 0 R/Info 36 0 R/ID[<3D84D5219073404D8FCDF268E1ED1BF4><3D84D5219073404D8FCDF268E1ED1BF4>] /Prev 160714/XRefStm 160062>>
        "};

        let (_, direct) = parse_plain(input).unwrap();
        let (_, trailer) = Trailer::extract(input).unwrap();

        assert_eq!(direct, trailer);

        let Trailer { dict, refs } = trailer;

        assert!(refs.is_empty());

        assert_eq!(dict.size, 194);
        assert_eq!(dict.root, TypedReference::new(1, 0));
        assert_eq!(dict.info, TypedReference::new(36, 0));
        assert_eq!(dict.prev, Some(160714));
    }

    #[test]
    fn xref_stream_trailer() {
        let input = include_bytes!("../../../../tests/objects/trailer_xref_stream.bin");

        let (_, direct) = parse_stream(input)
            .map_err(|e| e.map_input(Bytes::from))
            .unwrap();
        let (_, trailer) = Trailer::extract(input).unwrap();

        assert_eq!(direct, trailer);

        let Trailer { dict, refs } = trailer;

        assert_eq!(refs.len(), 2010);

        assert_eq!(dict.size, 92813);
        assert_eq!(dict.root, TypedReference::new(90794, 0));
        assert_eq!(dict.info, TypedReference::new(90792, 0));
        assert_eq!(dict.prev, Some(116));
    }
}
