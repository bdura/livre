use nom::IResult;

use crate::{
    RawDict,
    error::{ExtractionError, Result},
};

/// Extraction trait
pub trait Extract<'input>: Sized {
    fn extract(input: &'input [u8]) -> IResult<&'input [u8], Self>;
}

pub trait FromDict<'input>: Sized {
    fn from_dict(dict: RawDict<'input>) -> Result<Self>;
}

/// Parse trait, which mirrors the [`Extract`] trait.
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
