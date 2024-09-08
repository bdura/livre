use std::marker::PhantomData;

use winnow::{
    ascii::multispace1,
    combinator::{delimited, separated_pair, terminated, trace},
    error::{ContextError, ErrMode},
    BStr, PResult, Parser,
};

use crate::{extraction::extract, Extract};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ReferenceId {
    object: usize,
    generation: u16,
}

impl ReferenceId {
    pub fn new(object: usize, generation: u16) -> Self {
        Self { object, generation }
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
    id: ReferenceId,
    _mark: PhantomData<T>,
}

// We need to implement this manually because of the automatic trait bound on T.
impl<T> Clone for Reference<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            _mark: PhantomData,
        }
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

struct Indirect<T> {
    id: ReferenceId,
    inner: T,
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
}