use std::marker::PhantomData;

use winnow::{
    combinator::{alt, terminated, trace},
    BStr, PResult, Parser,
};

use super::ReferenceId;
use crate::extraction::{extract, Builder, Extract, Indirect};

/// A PDF reference to an *indirect object*.
///
/// PDF documents resort to a "random access" strategy to limit repetition and split large objects
/// into smaller atoms.
///
/// To that end, some objects will be represented by a `Reference`, indicating the object ID as
/// well as the generation number.
///
/// ## Note
///
/// I have still to understand what that means in practice... Although the definition is quite
/// simple, it looks like the generation number only takes two values: 0 or 65535.
/// Be that as it may, the `Reference` object in Livre proposes a type-safe implementation.
#[derive(Debug, PartialEq, Eq)]
pub struct Reference<T> {
    pub id: ReferenceId,
    _mark: PhantomData<T>,
}

/// We need to implement the [`Clone`] trait manually because of the automatic trait bound on `T`.
impl<T> Clone for Reference<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Reference<T> {}

impl<T> Extract<'_> for Reference<T> {
    fn extract(input: &mut &BStr) -> PResult<Self> {
        trace(
            "livre-reference",
            terminated(ReferenceId::extract, b" R").map(Self::from),
        )
        .parse_next(input)
    }
}

impl<T> From<ReferenceId> for Reference<T> {
    fn from(id: ReferenceId) -> Self {
        Self {
            id,
            _mark: PhantomData,
        }
    }
}

impl<T> From<(usize, u16)> for Reference<T> {
    fn from(id: (usize, u16)) -> Self {
        let id: ReferenceId = id.into();
        id.into()
    }
}

impl<'de, T> Reference<T>
where
    T: Extract<'de>,
{
    pub fn instantiate<B>(self, builder: &B) -> PResult<T>
    where
        B: Builder<'de>,
    {
        builder.build_reference(self)
    }

    pub fn build_indirect_object(&self, input: &mut &'de BStr) -> PResult<T> {
        let Indirect { id, inner } = extract(input)?;
        debug_assert_eq!(self.id, id, "the indirect id should be the expected id");
        Ok(inner)
    }
}

/// An optional reference, i.e. a type that may be represented directly of via a reference.
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum OptRef<T> {
    Ref(Reference<T>),
    Direct(T),
}

impl<'de, T> OptRef<T>
where
    T: Extract<'de>,
{
    /// Like the [`Reference`] type, [`OptRef`] declares an `instantiate` method to instantiate
    /// the underlying object, either by returning it directly (if the object was directly defined)
    /// or by having a [`Builder`] follow the reference.
    pub fn instantiate<B>(self, builder: &mut B) -> PResult<T>
    where
        B: Builder<'de>,
    {
        match self {
            Self::Ref(reference) => reference.instantiate(builder),
            Self::Direct(inner) => Ok(inner),
        }
    }
}

impl<'de, T> Extract<'de> for OptRef<T>
where
    T: Extract<'de>,
{
    fn extract(input: &mut &'de BStr) -> PResult<Self> {
        trace("livre-optref", move |i: &mut &'de BStr| {
            alt((
                Reference::extract.map(Self::Ref),
                T::extract.map(Self::Direct),
            ))
            .parse_next(i)
        })
        .parse_next(input)
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use crate::extraction::extract;

    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(b"0 0 R", (0, 0))]
    #[case(b"10 0 R", (10, 0))]
    #[case(b"10 10 R", (10, 10))]
    fn reference(#[case] input: &[u8], #[case] expected: impl Into<Reference<()>>) {
        let result = extract(&mut input.as_ref()).unwrap();
        assert_eq!(expected.into(), result);
    }

    #[rstest]
    #[case(b"0 0 R", b"0 0 obj\n10\nendobj", 10)]
    #[case(b"0 0 R", b"0 0 obj\ntrue\nendobj", true)]
    #[case(b"10 0 R", b"10 0 obj\n 10  true \nendobj", (10, true))]
    fn extraction<'de, T>(#[case] input: &[u8], #[case] source: &'de [u8], #[case] expected: T)
    where
        T: Extract<'de> + Debug + PartialEq,
    {
        let reference = Reference::extract(&mut input.as_ref()).unwrap();
        let extracted = reference
            .build_indirect_object(&mut source.as_ref())
            .unwrap();
        assert_eq!(expected, extracted)
    }

    #[rstest]
    #[case(b"0 0 R", OptRef::Ref(Reference::<u16>::from((0, 0))))]
    #[case(b"0", OptRef::Direct(0u16))]
    #[case(b"0 10", OptRef::Direct((0, 10)))]
    fn opt_ref<'de, T>(#[case] input: &'de [u8], #[case] expected: OptRef<T>)
    where
        T: Extract<'de> + Debug + PartialEq,
    {
        let result = extract(&mut input.as_ref()).unwrap();
        assert_eq!(expected, result);
    }
}
