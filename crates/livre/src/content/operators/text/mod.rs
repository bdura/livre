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

use crate::content::state::{TextObject, TextStateParameters};

use super::Operator;

/// Defines operations that act on the [text state parameters](TextStateParameters) alone:
/// the font (selected by the [`Tf` operator](SetFontAndFontSize)), spacing, scaling,
/// leading, rendering mode and rise. These parameters are part of the graphics state, so
/// these operators apply outside a text object as well as inside one.
#[enum_dispatch]
pub trait TextStateOperation: Sized {
    fn apply(self, parameters: &mut TextStateParameters);
}

/// Defines operations that move the text matrix of a [`TextObject`]. `TD` also sets the
/// leading, through the text object's parameters.
#[enum_dispatch]
pub trait TextPositioningOperation: Sized {
    fn apply(self, text_object: &mut TextObject);
}

/// Defines operations that show text within a [`TextObject`]. `'` and `"` move to the
/// next line first, and `"` also sets the word and character spacing.
#[enum_dispatch]
pub trait TextShowingOperation: Sized {
    fn apply(self, text_object: &mut TextObject);
}

/// Abstraction over any text operator, grouped by the category it belongs to.
#[derive(Debug, Clone, PartialEq)]
pub enum TextOperator {
    TextShowingOperator(TextShowingOperator),
    TextPositioningOperator(TextPositioningOperator),
    TextStateOperator(TextStateOperator),
}

impl TextOperator {
    /// Apply the operator inside a text object, dispatching to its category: text state
    /// operators act on the text object's parameters.
    pub fn apply(self, text_object: &mut TextObject) {
        match self {
            Self::TextShowingOperator(op) => op.apply(text_object),
            Self::TextPositioningOperator(op) => op.apply(text_object),
            Self::TextStateOperator(op) => op.apply(text_object.parameters),
        }
    }
}

impl From<TextShowingOperator> for TextOperator {
    fn from(op: TextShowingOperator) -> Self {
        Self::TextShowingOperator(op)
    }
}

impl From<TextPositioningOperator> for TextOperator {
    fn from(op: TextPositioningOperator) -> Self {
        Self::TextPositioningOperator(op)
    }
}

impl From<TextStateOperator> for TextOperator {
    fn from(op: TextStateOperator) -> Self {
        Self::TextStateOperator(op)
    }
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
