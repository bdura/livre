//! Livre organises the entire parsing logic around the [`Extract`] trait,
//! which defines a way for the type to extract itself from a stream of bytes,
//! consuming the input.

use livre_derive::FromRawDict;
use special::RawDict;
use winnow::{BStr, PResult, Parser};

mod primitives;
mod special;
mod utilities;

pub use special::Name;

/// The [`Extract`] trait marks a type as extractable from a stream of bytes,
/// without any context. Not object safe.
///
/// TODO: add a `Build` trait that can follow references as they arise
pub trait Extract<'de>: Sized {
    fn extract(input: &mut &'de BStr) -> PResult<Self>;

    /// Consume the input, without trying to parse. Especially useful for
    /// struct/map parsing, since we just need to extract the *bytes* associated
    /// with the type.
    ///
    /// Some types (if not all) may benefit from using a dedicated logic.
    fn recognize(input: &mut &'de BStr) -> PResult<&'de [u8]> {
        Self::extract.take().parse_next(input)
    }
}

/// Direct extraction. Most of the time the type can be inferred from
/// context, making this function very handy.
pub fn extract<'de, T>(input: &mut &'de BStr) -> PResult<T>
where
    T: Extract<'de>,
{
    T::extract(input)
}

/// The `FromRawDict` trait allows for the construction of complext
/// types using a pre-parsed dictionary.
///
/// Any type that is `FromRawDict` is trivially [`Extract`]
pub trait FromRawDict<'de>: Sized {
    fn from_raw_dict(dict: &mut RawDict<'de>) -> PResult<Self>;
}

impl<'de, T> Extract<'de> for T
where
    T: FromRawDict<'de>,
{
    fn extract(input: &mut &'de BStr) -> PResult<Self> {
        let mut dict = RawDict::extract(input)?;
        let result = Self::from_raw_dict(&mut dict)?;
        Ok(result)
    }
}
