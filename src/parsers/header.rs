use nom::{bytes::complete::tag, IResult};

use super::{comment::parse_comment, utilities::take_whitespace};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Version {
    Pdf10,
    Pdf11,
    Pdf12,
    Pdf13,
    Pdf14,
    Pdf15,
    Pdf16,
    Pdf17,
    Pdf20,
}

impl Version {
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, comment) = parse_comment(input)?;

        let (comment, _) = tag(b"PDF-")(comment)?;

        let version = match comment {
            b"1.0" => Self::Pdf10,
            b"1.1" => Self::Pdf11,
            b"1.2" => Self::Pdf12,
            b"1.3" => Self::Pdf13,
            b"1.4" => Self::Pdf14,
            b"1.5" => Self::Pdf15,
            b"1.6" => Self::Pdf16,
            b"1.7" => Self::Pdf17,
            b"2.0" => Self::Pdf20,
            _ => panic!("Unrecognized version: {}", String::from_utf8_lossy(comment)),
        };

        Ok((input, version))
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Header {
    pub version: Version,
    pub binary: bool,
}

impl Header {
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, version) = Version::parse(input)?;
        let (input, _) = take_whitespace(input)?;

        if let Ok((input, comment)) = parse_comment(input) {
            let binary = comment.iter().all(|&v| v >= 128);
            Ok((input, Self { version, binary }))
        } else {
            let binary = false;
            Ok((input, Self { version, binary }))
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn version() {
        let (_, version) = Version::parse(b"%PDF-1.6\n").unwrap();
        assert_eq!(version, Version::Pdf16);
    }

    #[test]
    fn version_from_pdf() {
        let pdf_bytes = include_bytes!("../../examples/text.pdf");
        let (_, version) = Version::parse(pdf_bytes).unwrap();
        assert_eq!(version, Version::Pdf14);
    }

    #[test]
    fn header_from_pdf() {
        let pdf_bytes = include_bytes!("../../examples/text.pdf");
        let (_, header) = Header::parse(pdf_bytes).unwrap();
        assert_eq!(
            header,
            Header {
                version: Version::Pdf14,
                binary: true
            }
        );
    }
}
