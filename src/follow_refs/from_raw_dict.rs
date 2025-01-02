use winnow::PResult;

use crate::extraction::{extract, RawDict};

use super::{Build, Builder};

pub trait BuildFromRawDict<'de>: Sized {
    fn build_from_raw_dict<B>(dict: &mut RawDict<'de>, builder: &B) -> PResult<Self>
    where
        B: Builder<'de>;
}

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
