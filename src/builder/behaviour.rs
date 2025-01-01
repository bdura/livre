use winnow::{
    error::{ContextError, ErrMode},
    BStr, PResult,
};

use super::BuilderParser;
use crate::extraction::{extract, Extract, Indirect, RawDict, Reference, ReferenceId};

/// Trait that can follow references.
///
/// A `Builder` holds every information to follow references and extract indirect objects.
/// It usually involves a mapping between [`ReferenceId`]s and their locations within the
/// input file.
pub trait Builder<'de>: Sized {
    /// Follow a reference and provide an (optional) pointer to the start of the indirect object.
    ///
    /// This is the entrypoint for the builder. This method provides the stream slice
    /// that describes the referenced entity. It returns an optional in case the reference
    /// is unknown to the builder.
    fn follow_reference(&self, reference_id: ReferenceId) -> Option<&'de BStr>;

    /// Build an object from the input. Direct analogue to the [`extract`] function.
    fn build<T>(&self, input: &mut &'de BStr) -> PResult<T>
    where
        T: Build<'de>,
    {
        T::build(input, self)
    }

    /// Follow a reference and extract it directly.
    ///
    /// This method checks that the reference is known to the builder, and returns a parsing error
    /// if that is not the case. It includes the mechanism to extract a *indirect object*.
    ///
    /// This method is usually the one that is used in practice.
    fn build_reference<T>(&self, Reference { id, .. }: Reference<T>) -> PResult<T>
    where
        T: Build<'de>,
    {
        let input = &mut self
            .follow_reference(id)
            .ok_or(ErrMode::Cut(ContextError::new()))?;

        let Indirect {
            id: reference_id,
            inner,
        } = Indirect::parse(input, self.as_parser())?;

        debug_assert_eq!(reference_id, id);

        Ok(inner)
    }
}

/// Generalisation on the [`Extract`] trait, which allows the extraction logic to follow references.
pub trait Build<'de>: Sized {
    /// Build an object that rely on a reference, which would be instantiated with the help of the
    /// supplied `builder`.
    ///
    /// The [`Build`] trait, like the [`Extract`] trait, is a linear parser above all, hence we
    /// supply an `input`. References found during parsing, if any, are first parsed as such, and
    /// then instantiated by the `builder`.
    fn build<B>(input: &mut &'de BStr, builder: &B) -> PResult<Self>
    where
        B: Builder<'de>;
}

/// [`Extract`] types are trivially [`Build`], since there is no reference to follow.
impl<'de, T> Build<'de> for T
where
    T: Extract<'de>,
{
    fn build<B>(input: &mut &'de BStr, _: &B) -> PResult<Self>
    where
        B: Builder<'de>,
    {
        extract(input)
    }
}

/// The `BuildFromRawDict` trait is the exact equivalent of
/// [`FromRawDict`](crate::extraction::FromRawDict), only for types that are [`Build`].
///
/// This type allows Livre to define potentially flattened datastructures.
pub trait BuildFromRawDict<'de>: Sized {
    fn build_from_raw_dict<B>(dict: &mut RawDict<'de>, builder: &B) -> PResult<Self>
    where
        B: Builder<'de>;
}
