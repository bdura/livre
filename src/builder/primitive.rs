use winnow::{combinator::trace, BStr, PResult, Parser};

use crate::extraction::{extract, OptRef};

use super::{behaviour::BuildFromRawDict, Build, Builder, BuilderParser};

/// An eager build primitive. By wrapping a type into `Built`, you signal to Livre that the
/// associated field may be an reference that should be followed.
pub struct Built<T>(pub T);

impl<'de, T> Build<'de> for Built<T>
where
    T: Build<'de>,
{
    fn build<B>(input: &mut &'de BStr, builder: &B) -> PResult<Self>
    where
        B: Builder<'de>,
    {
        trace("livre-built", move |i: &mut &'de BStr| {
            let optref = OptRef::parse(i, builder.as_parser())?;

            match optref {
                OptRef::Direct(value) => Ok(value),
                OptRef::Ref(reference) => builder.build_reference(reference),
            }
        })
        .parse_next(input)
    }
}

/// An eager build primitive, targeting structs that implement [`BuildFromRawDict`].
///
/// This is necessary because Rust cannot generate blanket implementations on two separate traits.
/// By using `BuiltStruct`, we circumvent this "limitation".
///
/// Note that it would probably be easier to implement [`Build`] directly. This may be done in a
/// future version using [`livre_derive`].
pub struct BuiltStruct<T>(pub T);

impl<'de, T> Build<'de> for BuiltStruct<T>
where
    T: BuildFromRawDict<'de>,
{
    fn build<B>(input: &mut &'de BStr, builder: &B) -> PResult<Self>
    where
        B: Builder<'de>,
    {
        let mut dict = extract(input)?;
        T::build_from_raw_dict(&mut dict, builder).map(Self)
    }
}
