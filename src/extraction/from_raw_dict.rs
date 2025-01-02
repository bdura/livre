pub use livre_derive::FromRawDict;
use winnow::{BStr, PResult};

use super::{Extract, RawDict};

/// The `FromRawDict` trait allows for the construction of complex types using a pre-parsed
/// dictionary.
///
/// This type can be derived using the [`livre_derive`] helper crate.
pub trait FromRawDict<'de>: Sized {
    /// Build a type from a raw dictionary. Note that the supplied dict is not consumed.
    /// Rather, the method takes hold of a mutable reference to extract only the fields
    /// that are needed, removing them from the dictionary.
    ///
    /// This means that we can break a single [`RawDict`] into multiple structured objects,
    /// which is particularly useful for compound PDF objects such as [`Stream`](super::Stream)s.
    fn from_raw_dict(dict: &mut RawDict<'de>) -> PResult<Self>;
}

/// Any type that is [`FromRawDict`] is trivially [`Extract`]: you first extract the [`RawDict`],
/// and apply [`FromRawDict`].
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
