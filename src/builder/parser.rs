use winnow::{error::ContextError, BStr, PResult, Parser};

use super::{Build, Builder};

/// Extension trait for the [`Builder`] trait, declaring the `as_parser` method.
///
/// With this trait in scope, any builder can become a parser. The `as_parser` method takes a
/// shared reference to self, so you can re-use it multiple times.
pub trait BuilderParser: Sized {
    fn as_parser(&self) -> LivreBuilder<'_, Self> {
        LivreBuilder(self)
    }
}

/// Actual implementation of the [`BuilderParser`] trait on [`Builder`].
impl<'de, B> BuilderParser for B where B: Builder<'de> {}

/// `LivreBuilder` wraps a generic [`Builder`] type to make it implement [winnow's `Parser`](Parser) trait.
/// You should not have to create this type yourself. Instead, call [`as_parser`](BuilderParser::as_parser)
/// on the builder.
///
/// `LivreBuilder` merely defers parsing to the wrapped builder. Its value is in making it
/// compatible with the rest of the winnow ecosystem.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct LivreBuilder<'b, B>(&'b B);

impl<'de, T, B> Parser<&'de BStr, T, ContextError> for LivreBuilder<'_, B>
where
    B: Builder<'de>,
    T: Build<'de>,
{
    fn parse_next(&mut self, input: &mut &'de BStr) -> PResult<T, ContextError> {
        self.0.build(input)
    }
}
