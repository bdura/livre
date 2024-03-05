use crate::Extract;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct IntoString(pub String);

impl Extract<'_> for IntoString {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        let obj = String::from_utf8_lossy(input).into();
        Ok((b"", Self(obj)))
    }
}
