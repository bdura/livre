use winnow::{combinator::trace, BStr, PResult, Parser};

use crate::extraction::OptRef;

use super::{Build, Builder, BuilderParser};

/// An eager build primitive. By wrapping a type into `Built`, you signal to Livre that the
/// associated field may be an reference that should be followed.
pub struct Built<T>(pub T);

impl<T> Build for Built<T>
where
    T: Build,
{
    fn build<B>(input: &mut &BStr, builder: &B) -> PResult<Self>
    where
        B: Builder,
    {
        trace("livre-built", move |i: &mut &BStr| {
            let optref: OptRef<T> = builder.as_parser().parse_next(i)?;

            match optref {
                OptRef::Direct(value) => Ok(value),
                OptRef::Ref(reference) => builder.build_reference(reference),
            }
        })
        .map(Self)
        .parse_next(input)
    }
}
