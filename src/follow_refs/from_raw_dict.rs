use winnow::PResult;

use crate::extraction::{extract, RawDict};

use super::{Build, Builder};

/// This awkwardly named trait lets a type define how it can be extracted from a [`RawDict`]
/// instance. It is a generalisation of the [`FromRawDict`](crate::extraction::FromRawDict)
/// trait.
///
/// This indirection in a type's ability to [`Build`] itself lets us define more complex extraction
/// strategies, in particular derivable ones. Since a `BuildFromRawDict` type merely pops relevant
/// keys from a mutable reference to a [`RawDict`], we give more structure to otherwise flat
/// dictionary structures.
pub trait BuildFromRawDict<'de>: Sized {
    fn build_from_raw_dict<B>(dict: &mut RawDict<'de>, builder: &B) -> PResult<Self>
    where
        B: Builder<'de>;
}

/// [`BuildFromRawDict`] types are trivially [`Build`]:
///
/// 1. Extract the underlying [`RawDict`]
/// 2. Use it to build the type
impl<'de, T> Build<'de> for T
where
    T: BuildFromRawDict<'de>,
{
    fn build<B>(input: &mut &'de winnow::BStr, builder: &B) -> winnow::PResult<Self>
    where
        B: Builder<'de>,
    {
        let mut dict = extract(input)?;
        Self::build_from_raw_dict(&mut dict, builder)
    }
}
