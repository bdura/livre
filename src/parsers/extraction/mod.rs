use super::*;

pub mod error;
use error::{ExtractionError, Result};

pub mod primitives;
pub use primitives::map::Map;

mod utilities;
pub use utilities::{
    Angles, Brackets, DbgStr, DoubleAngles, HexBytes, LitBytes, OptRef, Parentheses, RawValue,
};

pub mod pdf;
pub use pdf::{Indirect, Name, Reference, TypedReference};

pub mod extraction;
pub use extraction::Extract;

pub mod encoding;

/// Re-export IResult. Avoids depending on `nom` for downstream crates.
pub use nom::IResult;

pub fn extract<'input, T: Extract<'input>>(input: &'input [u8]) -> IResult<&'input [u8], T> {
    T::extract(input)
}

pub fn parse<'input, T: Extract<'input>>(input: &'input [u8]) -> Result<T> {
    let (_, obj) = T::extract(input).map_err(ExtractionError::custom)?;
    Ok(obj)
}
