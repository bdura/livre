use nom::IResult;

use crate::error::{ParsingError, Result};

pub trait Extract: Sized {
    fn extract(input: &[u8]) -> IResult<&[u8], Self>;
}

pub trait Parse: Sized {
    fn extract<T>(self) -> IResult<Self, T>
    where
        T: Extract;
    fn parse<T>(self) -> Result<T, ParsingError>
    where
        T: Extract;
}

impl Parse for &[u8] {
    fn parse<T>(self) -> Result<T, ParsingError>
    where
        T: Extract,
    {
        let (_, obj) = T::extract(self).map_err(|_| ParsingError::NomError)?;
        Ok(obj)
    }

    fn extract<T>(self) -> IResult<Self, T>
    where
        T: Extract,
    {
        T::extract(self)
    }
}

mod boolean;
mod integers;
mod reals;
