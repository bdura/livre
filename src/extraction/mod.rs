//! Livre organises the entire parsing logic around the [`Extract`] trait,
//! which defines a way for the type to extract itself from a stream of bytes,
//! consuming the input.

pub use livre_derive::FromRawDict;
use winnow::{
    ascii::multispace1,
    combinator::terminated,
    error::{ContextError, ErrMode},
    BStr, PResult, Parser,
};

mod primitives;
mod special;
mod utilities;

pub use special::{
    HexadecimalString, Indirect, LiteralString, MaybeArray, Name, OptRef, RawDict, Reference,
    ReferenceId, Stream,
};

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

/// An `Builder` holds every information to follow indirect references
pub trait Builder<'de>: Sized {
    /// The entrypoint for the builder. This method provides the stream slice
    /// that describes the referenced entity.
    fn follow_reference(&self, reference_id: ReferenceId) -> Option<&'de BStr>;
    fn build_reference<T>(&self, Reference { id, .. }: Reference<T>) -> PResult<T>
    where
        T: Build<'de>,
    {
        let mut input = self
            .follow_reference(id)
            .ok_or(ErrMode::Cut(ContextError::new()))?;

        let reference_id =
            terminated(ReferenceId::extract, (b" obj", multispace1)).parse_next(&mut input)?;

        debug_assert_eq!(reference_id, id);

        T::build(&mut input, self)
    }
}

pub trait BuildFromRawDict<'de>: Sized {
    fn build_from_raw_dict<B>(raw_dict: &mut RawDict<'de>, builder: &B) -> PResult<Self>
    where
        B: Builder<'de>;
}

/// A `Build` type uses an extractor to follow references
pub trait Build<'de>: Sized {
    fn build<B>(input: &mut &'de BStr, builder: &B) -> PResult<Self>
    where
        B: Builder<'de>;
}

impl<'de, T> Build<'de> for T
where
    T: Extract<'de>,
{
    fn build<B>(input: &mut &'de BStr, _: &B) -> PResult<Self>
    where
        B: Builder<'de>,
    {
        extract(input)
    }
}
