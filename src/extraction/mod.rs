use winnow::{BStr, PResult};

mod primitives;
mod utilities;

/// The [`Extract`] trait marks a type as extractable from a stream of bytes,
/// without any context.
///
/// TODO: add a `Build` trait that can follow references as they arise
pub trait Extract<'de>: Sized {
    fn extract(input: &mut &'de BStr) -> PResult<Self>;
}

/// Direct extraction. Most of the time the type can be inferred from
/// context, making this function very handy.
pub fn extract<'de, T>(input: &mut &'de BStr) -> PResult<T>
where
    T: Extract<'de>,
{
    T::extract(input)
}
