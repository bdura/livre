use std::marker::PhantomData;

use winnow::{
    combinator::{alt, terminated, trace},
    BStr, PResult, Parser,
};

use super::ReferenceId;
use crate::{
    extraction::Extract,
    follow_refs::{Build, Builder, BuilderParser},
};

/// A PDF reference to an *indirect object*.
///
/// In Livre, the `Reference` object is generic over the type of the indirect object, making the
/// entire interface type-safe. You can "opt out" of type safety by using an
/// [`Object`](crate::extraction::Object) type, which can represent any PDF object.
///
/// PDF documents resort to a "random access" strategy to limit repetition and split large objects
/// into smaller atoms. Indirect objects are also used when a property cannot be known in advance.
/// For instance, in the general case a PDF generator may choose to represent the `Length` key in
/// the [`Stream` dictionary](crate::extraction::Stream) with an indirect object to allow writing
/// the content to disk without having to know the serialised length in advance.
///
/// To that end, some objects are represented by a `Reference`, indicating the object ID
/// as well as the generation number. These are represented together in Livre, through the
/// [`ReferenceId`] type.
///
/// In practice, it looks like the generation number is not useful unless we are interested in
/// retrieving the PDF history. Livre tracks it in case we want to develop that capability down the
/// line.
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

/// An optional reference, i.e. a type that may be represented directly of via a reference.
///
/// Many properties in the PDF specification are optionally represented through an indirect object,
/// making this type extremely valuable.
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum OptRef<T> {
    Ref(Reference<T>),
    Direct(T),
}

impl<'de, T> Extract<'de> for OptRef<T>
where
    T: Extract<'de>,
{
    fn extract(input: &mut &'de BStr) -> PResult<Self> {
        trace(
            "livre-optref",
            alt((
                Reference::extract.map(Self::Ref),
                T::extract.map(Self::Direct),
            )),
        )
        .parse_next(input)
    }
}

impl<T> Build for OptRef<T>
where
    T: Build,
{
    fn build<B>(input: &mut &BStr, builder: &B) -> PResult<Self>
    where
        B: Builder,
    {
        trace(
            "livre-optref",
            alt((
                Reference::extract.map(Self::Ref),
                builder.as_parser().map(Self::Direct),
            )),
        )
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
