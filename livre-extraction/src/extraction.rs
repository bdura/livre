use nom::IResult;

use crate::{error::Result, RawDict};

/// Extraction trait
pub trait Extract<'input>: Sized {
    fn extract(input: &'input [u8]) -> IResult<&'input [u8], Self>;
}

pub trait FromDictRef<'input>: Sized {
    fn from_dict_ref(dict: &mut RawDict<'input>) -> Result<Self>;
}

pub trait FromDict<'input>: Sized {
    fn from_dict(dict: RawDict<'input>) -> Result<Self>;
}

/// Parse trait, which mirrors the [`Extract`] trait.
pub trait Parse<'input>: Sized {
    fn extract<T: Extract<'input>>(self) -> IResult<Self, T>;
    fn parse<T: Extract<'input>>(self) -> Result<T>;
}
