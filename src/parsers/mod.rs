mod header;
use std::collections::HashMap;

pub use header::Header;

mod crossref;
pub use crossref::CrossRef;

mod trailer;
pub use trailer::Trailer;

pub mod object;
use nom::{
    bytes::complete::tag,
    combinator::opt,
    error::Error,
    multi::{many0, separated_list1},
    sequence::{preceded, tuple},
    IResult,
};
pub use object::Object;

mod comment;
pub use comment::Comment;

use crate::parsers::utilities::{parse_digits, take_whitespace};

pub mod utilities;

#[derive(Debug, Clone, PartialEq)]
pub struct Pdf {
    pub header: Header,
    pub updates: Vec<Update>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Update {
    pub body: Vec<Object>,
    pub cross_refs: HashMap<usize, CrossRef>,
    pub trailer: Trailer,
    pub startxref: Option<usize>,
}

impl Update {
    fn parse_body(input: &[u8]) -> IResult<&[u8], Vec<Object>> {
        let (input, body) = many0(Object::parse_referenced)(input)?;
        let objects = body.into_iter().map(|(_, o)| o).collect();
        Ok((input, objects))
    }

    fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, body) = Self::parse_body(input).unwrap();
        let (input, cross_refs) = opt(preceded(take_whitespace, CrossRef::parse))(input).unwrap();

        let (input, trailer) = opt(preceded(take_whitespace, Trailer::parse))(input).unwrap();
        let (input, startxref) = opt(tuple((
            take_whitespace,
            tag(b"startxref"),
            take_whitespace,
            parse_digits,
        )))(input)
        .unwrap();
        let (input, comment) = preceded(take_whitespace, Comment::parse)(input)?;

        assert_eq!(comment.0, "%EOF");

        let startxref = startxref.map(|(_, _, _, v)| v);
        let cross_refs = cross_refs.unwrap_or_default();
        let update = Self {
            body,
            cross_refs,
            trailer: trailer.unwrap_or_default(),
            startxref,
        };

        Ok((input, update))
    }
}

impl Pdf {
    fn parse_pdf(input: &[u8]) -> Result<Self, nom::Err<Error<&[u8]>>> {
        let (input, header) = Header::parse(input)?;
        let (input, _) = take_whitespace(input)?;

        let (_, updates) = separated_list1(take_whitespace, Update::parse)(input).unwrap();

        Ok(Self { header, updates })
    }
    pub fn parse(input: &[u8]) -> Result<Self, String> {
        let pdf = Self::parse_pdf(input).map_err(|e| format!("Error while parsing: {e}"))?;
        Ok(pdf)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    use indoc::indoc;

    // #[test]
    // fn text_pdf() {
    //     // let pdf_bytes = include_bytes!("../../examples/text.pdf");
    //     let pdf_bytes = include_bytes!("../../resource/ISO_32000-2-2020_sponsored.pdf");
    //     let pdf = Pdf::parse(pdf_bytes).unwrap();

    //     assert_eq!(
    //         pdf.header,
    //         Header {
    //             version: Version::Pdf17,
    //             binary: true
    //         }
    //     );
    // }

    #[test]
    fn minimal() {
        let input = indoc! {b"
            %PDF-1.7
            90793 0 obj
            <</Linearized 1/L 14721088/O 90795/E 100639/N 1003/T 14709646/H [ 3414 9955]>>
            endobj
            
            90824 0 obj
            <</DecodeParms<</Columns 5/Predictor 12>>/Filter/FlateDecode/ID[<2B551D2AFE52654494F9720283CFF1C4><3CDA8BB6D5834E41A5E2AA16C35E4C47>]/Index[90793 1014]/Info 90792 0 R/Length 5/Prev 14709647/Root 90794 0 R/Size 91807/Type/XRef/W[1 3 1]>>stream
            12345
            endstream
            endobj
            startxref
            0
            %%EOF
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            
            91806 0 obj
            <</B 18672/C 18650/E 18634/Filter/FlateDecode/I 18698/Length 9818/O 18596/S 17836/T 18540/V 18612>>
            endobj
        "};

        Pdf::parse(input).unwrap();
    }
}
