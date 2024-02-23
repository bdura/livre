use nom::{bytes::complete::tag, IResult};

use crate::utilities::{parse_comment, take_whitespace};

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

    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(b"%PDF-1.0\n", Version::Pdf10)]
    #[case(b"%PDF-1.1\n", Version::Pdf11)]
    #[case(b"%PDF-1.2\n", Version::Pdf12)]
    #[case(b"%PDF-1.3\n", Version::Pdf13)]
    #[case(b"%PDF-1.4\n", Version::Pdf14)]
    #[case(b"%PDF-1.5\n", Version::Pdf15)]
    #[case(b"%PDF-1.6\n", Version::Pdf16)]
    #[case(b"%PDF-1.7\n", Version::Pdf17)]
    #[case(b"%PDF-2.0\n", Version::Pdf20)]
    fn version(#[case] input: &[u8], #[case] expected: Version) {
        let (_, version) = Version::parse(input).unwrap();
        assert_eq!(version, expected);
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
