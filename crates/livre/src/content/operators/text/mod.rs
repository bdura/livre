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

use crate::content::{
    error::Result,
    state::{TextObject, TextStateParameters},
    Fonts,
};

use super::Operator;

/// Defines operations that act on the [text state parameters](TextStateParameters) alone:
/// the font (selected by the [`Tf` operator](SetFontAndFontSize)), spacing, scaling,
/// leading, rendering mode and rise. These parameters are part of the graphics state, so
/// these operators apply outside a text object as well as inside one.
#[enum_dispatch]
pub trait TextStateOperation: Sized {
    /// Update the text state parameters: the operator's effect lasts until another
    /// operator overrides it or `Q` restores an earlier state. `Tf` looks its font up in
    /// `fonts`, the page's font resources, and the parameters keep a borrow of it.
    ///
    /// # Errors
    ///
    /// [`ContentError::UnknownFont`](crate::content::ContentError::UnknownFont) if `Tf`
    /// names a font absent from `fonts`. The parameters are left unchanged.
    fn apply<'f>(self, parameters: &mut TextStateParameters<'f>, fonts: &'f Fonts) -> Result<()>;
}

/// Defines operations that move the text matrix of a [`TextObject`]. `TD` also sets the
/// leading, through the text object's parameters.
#[enum_dispatch]
pub trait TextPositioningOperation: Sized {
    /// Update the text object's text matrix. `TD` also sets the leading in its parameters.
    fn apply(self, text_object: &mut TextObject);
}

/// Defines operations that show text within a [`TextObject`]. `'` and `"` move to the
/// next line first, and `"` also sets the word and character spacing.
#[enum_dispatch]
pub trait TextShowingOperation: Sized {
    /// Buffer the shown string, or `TJ` array, in the text object for its stream to yield
    /// as runs. `'` and `"` move the text matrix first, and `"` sets the spacing parameters.
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
    /// Apply the operator inside a text object, dispatching to its category. Text state
    /// operators mutate the text state parameters the object borrows from the graphics
    /// state, so their effect persists past `ET`.
    ///
    /// # Errors
    ///
    /// [`ContentError::UnknownFont`](crate::content::ContentError::UnknownFont) if `Tf`
    /// names a font absent from the text object's fonts.
    pub fn apply(self, text_object: &mut TextObject) -> Result<()> {
        match self {
            Self::TextShowingOperator(op) => op.apply(text_object),
            Self::TextPositioningOperator(op) => op.apply(text_object),
            Self::TextStateOperator(op) => {
                return op.apply(text_object.parameters, text_object.fonts)
            }
        }
        Ok(())
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
