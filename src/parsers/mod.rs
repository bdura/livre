mod header;
pub use header::Header;

pub mod object;

mod comment;
pub use comment::Comment;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pdf {
    pub header: Header,
}

impl Pdf {
    pub fn parse(input: &[u8]) -> Result<Self, String> {
        let (_, header) = Header::parse(input).map_err(|e| format!("Error while parsing: {e}"))?;
        Ok(Self { header })
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
            pdf,
            Pdf {
                header: Header {
                    version: Version::Pdf14,
                    binary: true
                }
            }
        );
    }
}
