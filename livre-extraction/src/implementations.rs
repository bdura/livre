use nom::IResult;

use crate::{
    error::{ExtractionError, Result}, Extract, FromDict, FromDictRef, Parse, RawDict
};

impl<'input, T> FromDict<'input> for T
where
    T: FromDictRef<'input>,
{
    fn from_dict(mut dict: RawDict<'input>) -> Result<Self> {
        T::from_dict_ref(&mut dict)
    }
}

impl<'input> Parse<'input> for &'input [u8] {
    fn parse<T: Extract<'input>>(self) -> Result<T> {
        let (_, obj) = T::extract(self).map_err(|_| ExtractionError::Unknown)?;
        Ok(obj)
    }

    fn extract<T: Extract<'input>>(self) -> IResult<Self, T> {
        T::extract(self)
    }
}
