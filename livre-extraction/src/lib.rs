pub mod error;
use error::{ExtractionError, Result};

pub mod primitives;
pub use primitives::map::Map;

mod utilities;
pub use utilities::{
    Angles, Brackets, DbgStr, DoubleAngles, MaybeArray, NoOp, OptRef, Parentheses, RawDict,
};

pub mod pdf;
pub use pdf::{HexString, Indirect, Name, Reference, TypedReference};

pub mod extraction;
pub use extraction::{Extract, FromDict, FromDictRef};

pub use livre_derive::{Extract, FromDictRef};

/// Re-export IResult. Avoids depending on `nom` for downstream crates.
pub use nom::IResult;

pub fn extract<'input, T: Extract<'input>>(input: &'input [u8]) -> IResult<&'input [u8], T> {
    T::extract(input)
}

pub fn from_dict_ref<'input, T: FromDictRef<'input>>(dict: &mut RawDict<'input>) -> Result<T> {
    T::from_dict_ref(dict)
}

pub fn parse<'input, T: Extract<'input>>(input: &'input [u8]) -> Result<T> {
    let (_, obj) = T::extract(input).map_err(|_| ExtractionError::Unknown)?;
    Ok(obj)
}
