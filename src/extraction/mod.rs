use nom::IResult;

use crate::error::{ParsingError, Result};

pub trait Extract<'input>: Sized {
    fn extract(input: &'input [u8]) -> IResult<&'input [u8], Self>;
}

pub trait Parse<'input>: Sized {
    fn extract<T: Extract<'input>>(self) -> IResult<Self, T>;
    fn parse<T: Extract<'input>>(self) -> Result<T, ParsingError>;
}

impl<'input> Parse<'input> for &'input [u8] {
    fn parse<T: Extract<'input>>(self) -> Result<T, ParsingError> {
        let (_, obj) = T::extract(self).map_err(|_| ParsingError::NomError)?;
        Ok(obj)
    }

    fn extract<T: Extract<'input>>(self) -> IResult<Self, T> {
        T::extract(self)
    }
}

mod boolean;
mod dictionary;
mod numbers;
mod string;
