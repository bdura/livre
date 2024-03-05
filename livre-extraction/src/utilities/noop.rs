use crate::{Extract, FromDictRef};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct NoOp;

impl FromDictRef<'_> for NoOp {
    fn from_dict_ref(_: &mut crate::RawDict<'_>) -> crate::error::Result<Self> {
        Ok(Self)
    }
}

impl Extract<'_> for NoOp {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        Ok((input, Self))
    }
}
