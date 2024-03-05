pub mod error;

pub mod primitives;
pub use primitives::map::Map;

mod utilities;
pub use utilities::{
    Angles, Brackets, DoubleAngles, MaybeArray, NoOp, OptRef, Parentheses, RawDict,
};

pub mod pdf;
pub use pdf::{HexString, Indirect, Name, Reference};

pub mod extraction;
pub use extraction::{Extract, FromDict, FromDictRef};

pub use livre_derive::{Extract, FromDict, FromDictRef};

pub mod parsing;
pub use parsing::Parse;

/// Re-export IResult. Avoids depending on `nom` for downstream crates.
pub use nom::IResult;

pub fn extract<'input, T: Extract<'input>>(input: &'input [u8]) -> IResult<&'input [u8], T> {
    T::extract(input)
}
