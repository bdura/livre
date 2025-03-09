//! Text operators, roughly organised following the PDF specification's hierarchy.

mod delimiters;
mod positioning;
mod showing;
mod state;

pub use delimiters::{BeginText, EndText};
use enum_dispatch::enum_dispatch;
pub use positioning::{
    MoveByOffset, MoveByOffsetAndSetLeading, MoveToNextLine, SetTextMatrix, TextPositioningOperator,
};
pub use showing::{
    MoveToNextLineAndShowText, MoveToNextLineAndShowTextWithSpacing, ShowText, ShowTextArray,
    TextArrayElement, TextShowingOperator,
};
pub use state::{
    SetCharacterSpacing, SetFontAndFontSize, SetHorizontalScaling, SetTextLeading,
    SetTextRenderingMode, SetTextRise, SetWordSpacing, TextStateOperator,
};

use crate::content::state::{TextMatrix, TextObject, TextStateParameters};

use super::Operator;

/// Defines operations that act on the text state.
#[enum_dispatch]
pub trait TextOperation: Sized {
    fn apply(self, text_object: &mut TextObject);
}

/// Operators that can be applied before the text object is fully constructed.
///
/// By definition, these include the [`Tf` operator](SetFontAndFontSize),
/// since the text state cannot exist without a font and a font size.
///
/// `PreTextOperation` types are automatically [`TextOperation`] thanks to a blanket
/// implementation.
#[enum_dispatch]
pub trait PreTextOperation: Sized {
    fn preapply(self, matrix: &mut TextMatrix, parameters: &mut TextStateParameters);
}

impl<T> TextOperation for T
where
    T: PreTextOperation,
{
    fn apply(self, text_object: &mut TextObject) {
        self.preapply(&mut text_object.matrix, &mut text_object.parameters);
    }
}

/// Abstraction over any text operator. `TextOperator` implements [`TextOperation`],
/// allowing it to modify a text object.
#[derive(Debug, Clone, PartialEq)]
#[enum_dispatch(TextOperation)]
pub enum TextOperator {
    TextShowingOperator(TextShowingOperator),
    TextPositioningOperator(TextPositioningOperator),
    TextStateOperator(TextStateOperator),
}

macro_rules! chain_into {
    ($from:ty => $via:ident => $into:ty) => {
        impl From<$from> for $into {
            fn from(op: $from) -> Self {
                Self::$via(op.into())
            }
        }
    };
    ($via:ident => $into:ty; $($from:ty,)+) => {
        $(chain_into!($from => $via => $into);)+
    };
}

chain_into!(TextStateOperator => TextOperator;
    SetCharacterSpacing,
    SetWordSpacing,
    SetHorizontalScaling,
    SetTextLeading,
    SetFontAndFontSize,
    SetTextRenderingMode,
    SetTextRise,
);
chain_into!(TextShowingOperator => TextOperator;
    ShowTextArray,
    ShowText,
    MoveToNextLineAndShowText,
    MoveToNextLineAndShowTextWithSpacing,
);
chain_into!(TextPositioningOperator => TextOperator;
    MoveByOffset,
    MoveByOffsetAndSetLeading,
    SetTextMatrix,
    MoveToNextLine,
);

impl<T> From<T> for Operator
where
    T: Into<TextOperator>,
{
    fn from(op: T) -> Self {
        Operator::Text(op.into())
    }
}
