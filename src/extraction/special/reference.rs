use std::marker::PhantomData;

use winnow::{
    ascii::multispace1,
    combinator::{alt, delimited, separated_pair, terminated, trace},
    error::{ContextError, ErrMode},
    BStr, PResult, Parser,
};

use crate::{
    extraction::{extract, Builder},
    Build, Extract,
};

/// An ID that uniquely identifies an object and its version.
///
/// In practice, it looks like the [`object`](Self::object) field alone
/// should be enough for text extraction since the XRef dictionary is
/// updated such that only the latest version of a given object is referenced.
///
/// In the future we *might* want to look at the document's history,
/// hence the [`ReferenceId`] keeps the generation number.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct ReferenceId {
    object: usize,
    generation: u16,
}

impl ReferenceId {
    pub fn new(object: usize, generation: u16) -> Self {
        Self { object, generation }
    }

    pub fn first(object: usize) -> Self {
        Self {
            object,
            generation: 0,
        }
    }
}

impl Extract<'_> for ReferenceId {
    fn extract(input: &mut &BStr) -> PResult<Self> {
        trace(
            "livre-reference-id",
            separated_pair(extract, b' ', extract)
                .map(|(object, generation)| Self::new(object, generation)),
        )
        .parse_next(input)
    }
}

impl From<(usize, u16)> for ReferenceId {
    fn from((object, generation): (usize, u16)) -> Self {
        Self::new(object, generation)
    }
}

/// PDF documents resort to a "random access" strategy to limit repetition and split large objects
/// into smaller atoms.
///
/// To that end, some objects will be represented by a `Reference`, indicating the object ID as
/// well as the generation number.
///
/// I have still to understand what that means in practice... Although the definition is quite
/// simple, it looks like the generation number only takes two values: 0 or 65535.
/// Be that as it may, the `Reference` object in Livre proposes a type-safe implementation.
#[derive(Debug, PartialEq, Eq)]
pub struct Reference<T> {
    pub id: ReferenceId,
    _mark: PhantomData<T>,
}

// We need to implement this manually because of the automatic trait bound on T.
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

impl<T> Reference<T>
where
    T: for<'de> Build<'de>,
{
    pub fn instantiate<'de, B>(self, builder: &B) -> PResult<T>
    where
        B: Builder<'de>,
    {
        builder.build_reference(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum OptRef<T> {
    Ref(Reference<T>),
    Direct(T),
}

impl<T> OptRef<T>
where
    T: for<'de> Build<'de>,
{
    pub fn instantiate<'de, B>(self, builder: &mut B) -> PResult<T>
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

/// The source for an indirect object, which can later be
/// referenced using a [`Reference`].
pub struct Indirect<T> {
    pub id: ReferenceId,
    pub inner: T,
}

impl<'de, T> Extract<'de> for Indirect<T>
where
    T: Extract<'de>,
{
    fn extract(input: &mut &'de BStr) -> PResult<Self> {
        trace(
            "livre-indirect",
            (
                ReferenceId::extract,
                delimited((b" obj", multispace1), T::extract, (multispace1, b"endobj")),
            )
                .map(|(id, inner)| Self { id, inner }),
        )
        .parse_next(input)
    }
}

impl<'de, T> Reference<T>
where
    T: Extract<'de>,
{
    /// Instantiate a typed object from the relevant input slice.
    ///
    /// TODO: add an instantiation method, that uses the ReferenceId to
    /// get the input slice.
    pub fn extract_from_source(&self, input: &mut &'de BStr) -> PResult<T> {
        let Indirect { id, inner } = extract(input).unwrap();

        if id != self.id {
            return Err(ErrMode::Cut(ContextError::new()));
        }

        Ok(inner)
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

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
        let extracted = reference.extract_from_source(&mut source.as_ref()).unwrap();
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
