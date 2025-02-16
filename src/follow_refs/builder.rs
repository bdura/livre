use winnow::{
    error::{ContextError, ErrMode},
    BStr, PResult, Parser,
};

use crate::extraction::Reference;

use super::Build;

/// Trait that can follow references.
///
/// A `Builder` holds every information to follow references and extract indirect objects.
/// It usually involves a mapping between [`ReferenceId`](crate::extraction::ReferenceId)s
/// and their locations within the input file.
pub trait Builder: Sized {
    /// Build an object from the input. Direct analogue to the
    /// [`extract`](crate::extraction::extract) function.
    fn build<T>(&self, input: &mut &BStr) -> PResult<T>
    where
        T: Build,
    {
        T::build(input, self)
    }

    /// Follow a reference and extract it directly.
    ///
    /// This method checks that the reference is known to the builder, and returns a parsing error
    /// if that is not the case. It includes the mechanism to extract a *indirect object*.
    ///
    /// This method is usually the one that is used in practice.
    fn build_reference<T>(&self, reference: Reference<T>) -> PResult<T>
    where
        T: Build;
    //{
    //    let input = &mut self
    //        .follow_reference(id)
    //        .ok_or(ErrMode::Cut(ContextError::new()))?;
    //
    //    let Indirect {
    //        id: reference_id,
    //        inner,
    //    } = Indirect::parse(input, self.as_parser())?;
    //
    //    debug_assert_eq!(reference_id, id);
    //
    //    Ok(inner)
    //}
}

/// The unit type is a context-less builder, making `().as_parser` somewhat equivalent to
/// `extact`: it will simply error if there is any reference to instantiate.
impl Builder for () {
    fn build_reference<T>(&self, _: Reference<T>) -> PResult<T>
    where
        T: Build,
    {
        Err(ErrMode::Backtrack(ContextError::new()))
    }
}

/// Extension trait for the [`Builder`] trait, declaring the `as_parser` method.
///
/// With this trait in scope, any builder can become a parser. The `as_parser` method takes a
/// shared reference to self, so you can re-use it multiple times.
///
/// This is not added to the `Builder` trait directly, to keep it object-safe.
pub trait BuilderParser: Sized {
    fn as_parser(&self) -> LivreBuilder<'_, Self> {
        LivreBuilder(self)
    }
}

/// Actual implementation of the [`BuilderParser`] trait on [`Builder`].
impl<B> BuilderParser for B where B: Builder {}

/// `LivreBuilder` wraps a generic [`Builder`] type to make it implement [winnow's `Parser`](Parser) trait.
/// You should not have to create this type yourself. Instead, call [`as_parser`](BuilderParser::as_parser)
/// on the builder.
///
/// `LivreBuilder` merely defers parsing to the wrapped builder. Its value is in making it
/// compatible with the rest of the [`winnow`] ecosystem.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct LivreBuilder<'b, B>(&'b B);

impl<T, B> Parser<&BStr, T, ContextError> for LivreBuilder<'_, B>
where
    B: Builder,
    T: Build,
{
    fn parse_next(&mut self, input: &mut &BStr) -> PResult<T, ContextError> {
        self.0.build(input)
    }
}
