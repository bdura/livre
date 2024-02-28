use nom::IResult;

use crate::error::{ExtractionError, Result};

use super::Extract;

pub trait Parse<'input>: Sized {
    fn extract<T: Extract<'input>>(self) -> IResult<Self, T>;
    fn parse<T: Extract<'input>>(self) -> Result<T>;
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
