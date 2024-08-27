//! Livre organises the entire parsing logic around the [`Extract`] trait,
//! which defines a way for the type to extract itself from a stream of bytes,
//! consuming the input.

use winnow::{BStr, PResult, Parser};

mod pdf;
mod primitives;
mod utilities;

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
