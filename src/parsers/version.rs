use nom::{
    bytes::complete::{tag, take},
    character::complete::char,
    error::Error,
    IResult,
};

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
        let (input, _) = tag(b"%PDF-")(input)?;
        let (input, version) = take(3usize)(input)?;

        let version = match version {
            b"1.0" => Self::Pdf10,
            b"1.1" => Self::Pdf11,
            b"1.2" => Self::Pdf12,
            b"1.3" => Self::Pdf13,
            b"1.4" => Self::Pdf14,
            b"1.5" => Self::Pdf15,
            b"1.6" => Self::Pdf16,
            b"1.7" => Self::Pdf17,
            b"2.0" => Self::Pdf20,
            _ => panic!("Unrecognized version: {version:?}"),
        };

        Ok((input, version))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pdf {
    version: Version,
}

impl Pdf {
    pub fn parse(input: &[u8]) -> Result<Self, String> {
        let (_, version) =
            Version::parse(input).map_err(|e| format!("Error while parsing: {e}"))?;
        Ok(Self { version })
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn version() {
        let (_, version) = Version::parse(b"%PDF-1.6").unwrap();
        assert_eq!(version, Version::Pdf16);
    }

    #[test]
    fn version_from_pdf() {
        let pdf_bytes = include_bytes!("../../examples/text.pdf");
        let (_, version) = Version::parse(pdf_bytes).unwrap();
        assert_eq!(version, Version::Pdf14);
    }
}
