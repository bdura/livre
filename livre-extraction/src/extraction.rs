use nom::IResult;

use crate::{error::Result, RawDict};

/// Extraction trait
pub trait Extract<'input>: Sized {
    fn extract(input: &'input [u8]) -> IResult<&'input [u8], Self>;
}

/// Parses the struct from a [`RawDict`], taking the needed values from it.
pub trait FromDictRef<'input>: Sized {
    fn from_dict_ref(dict: &mut RawDict<'input>) -> Result<Self>;
}

pub trait FromDict<'input>: Sized {
    fn from_dict(dict: RawDict<'input>) -> Result<Self>;
}

impl<'input, T> FromDict<'input> for T
where
    T: FromDictRef<'input>,
{
    fn from_dict(mut dict: RawDict<'input>) -> Result<Self> {
        T::from_dict_ref(&mut dict)
    }
}
