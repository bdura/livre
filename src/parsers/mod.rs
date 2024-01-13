mod version;
use version::Version;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pdf {
    pub version: Version,
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
    fn text_pdf() {
        let pdf_bytes = include_bytes!("../../examples/text.pdf");
        let pdf = Pdf::parse(pdf_bytes).unwrap();
        assert_eq!(
            pdf,
            Pdf {
                version: Version::Pdf14
            }
        );
    }
}
