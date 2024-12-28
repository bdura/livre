mod id;
mod reference;

pub use id::ReferenceId;
pub use reference::{OptRef, Reference};

use winnow::{
    ascii::multispace1,
    combinator::{delimited, trace},
    error::{ContextError, ErrMode},
    BStr, PResult, Parser,
};

use crate::extraction::{extract, Build, Builder, Extract};

/// The source for an indirect object, which can later be referenced using a [`Reference`].
pub struct Indirect<T> {
    pub id: ReferenceId,
    pub inner: T,
}

impl<'de, T> From<(ReferenceId, T)> for Indirect<T>
where
    T: Extract<'de>,
{
    fn from((id, inner): (ReferenceId, T)) -> Self {
        Self { id, inner }
    }
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
                .map(Self::from),
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

/// Type representing a followed reference, i.e. a possible reference that has been built.
/// **Not in the PDF specification**.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct FollowedRef<T>(pub T);

impl<T> FollowedRef<T> {
    /// Extract the inner object.
    pub fn extract(self) -> T {
        self.0
    }
}

impl<'de, T> Build<'de> for FollowedRef<T>
where
    T: for<'a> Extract<'a>,
{
    fn build<B>(input: &mut &'de BStr, builder: &B) -> PResult<Self>
    where
        B: Builder<'de>,
    {
        let opt_ref: OptRef<T> = extract(input)?;
        let inner = opt_ref.instantiate(builder)?;
        Ok(Self(inner))
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
