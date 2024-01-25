mod header;
use std::collections::HashMap;

pub use header::Header;

mod crossref;
pub use crossref::CrossRef;

pub mod object;
use nom::{error::Error, multi::many0, IResult};
pub use object::Object;

mod comment;
pub use comment::Comment;

use crate::parsers::utilities::take_whitespace;

pub mod utilities;

#[derive(Debug, Clone, PartialEq)]
pub struct Pdf {
    pub header: Header,
    pub body: Vec<Object>,
    pub cross_refs: HashMap<usize, CrossRef>,
}

impl Pdf {
    fn parse_body(input: &[u8]) -> IResult<&[u8], Vec<Object>> {
        let (input, body) = many0(Object::parse_referenced)(input)?;
        let objects = body.into_iter().map(|(_, o)| o).collect();
        Ok((input, objects))
    }

    fn parse_pdf(input: &[u8]) -> Result<Self, nom::Err<Error<&[u8]>>> {
        let (input, header) = Header::parse(input)?;
        let (input, _) = take_whitespace(input)?;
        let (input, body) = Self::parse_body(input)?;
        let (input, _) = take_whitespace(input)?;
        let (_, cross_refs) = CrossRef::parse(input)?;

        Ok(Self {
            header,
            body,
            cross_refs,
        })
    }
    pub fn parse(input: &[u8]) -> Result<Self, String> {
        let pdf = Self::parse_pdf(input).map_err(|e| format!("Error while parsing: {e}"))?;
        Ok(pdf)
    }
}

#[cfg(test)]
mod tests {

    use crate::parsers::header::Version;

    use super::*;

    #[test]
    fn text_pdf() {
        let pdf_bytes = include_bytes!("../../examples/text.pdf");
        let pdf = Pdf::parse(pdf_bytes).unwrap();

        assert_eq!(
            pdf.header,
            Header {
                version: Version::Pdf14,
                binary: true
            }
        );
    }
}
