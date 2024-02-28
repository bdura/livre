use nom::IResult;

use crate::error::{ParsingError, Result};

pub trait Extract: Sized {
    fn extract(input: &[u8]) -> IResult<&[u8], Self>;
}

pub trait Parse: Sized {
    fn extract<T: Extract>(self) -> IResult<Self, T>;
    fn parse<T: Extract>(self) -> Result<T, ParsingError>;
}

impl Parse for &[u8] {
    fn parse<T: Extract>(self) -> Result<T, ParsingError> {
        let (_, obj) = T::extract(self).map_err(|_| ParsingError::NomError)?;
        Ok(obj)
    }

    fn extract<T: Extract>(self) -> IResult<Self, T> {
        T::extract(self)
    }
}

mod boolean;
mod numbers;
