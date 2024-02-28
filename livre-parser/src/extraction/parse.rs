use nom::IResult;

use crate::error::ParseError;

use super::Extract;

pub trait Parse<'input>: Sized {
    fn extract<T: Extract<'input>>(self) -> IResult<Self, T>;
    fn parse<T: Extract<'input>>(self) -> Result<T, ParseError>;
}

impl<'input> Parse<'input> for &'input [u8] {
    fn parse<T: Extract<'input>>(self) -> Result<T, ParseError> {
        let (_, obj) = T::extract(self).map_err(|_| ParseError::Unknown)?;
        Ok(obj)
    }

    fn extract<T: Extract<'input>>(self) -> IResult<Self, T> {
        T::extract(self)
    }
}
