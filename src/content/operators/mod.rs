mod accumulator;
mod text;

use thiserror::Error;

use text::{
    BeginText, EndText, MoveByOffset, MoveByOffsetAndSetLeading, MoveToNextLine,
    MoveToNextLineAndShowText, MoveToNextLineAndShowTextWithSpacing, SetCharacterSpacing,
    SetFontAndFontSize, SetHorizontalScaling, SetTextLeading, SetTextMatrix, SetTextRenderingMode,
    SetTextRise, SetWordSpacing, ShowText, ShowTextArray,
};
use winnow::error::{ContextError, ErrMode};

use crate::extraction::Name;

use super::arguments::Object;

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Operator {
    SetCharacterSpacing(SetCharacterSpacing),
    SetWordSpacing(SetWordSpacing),
    SetHorizontalScaling(SetHorizontalScaling),
    SetTextLeading(SetTextLeading),
    SetFontAndFontSize(SetFontAndFontSize),
    SetTextRenderingMode(SetTextRenderingMode),
    SetTextRise(SetTextRise),
    BeginText(BeginText),
    EndText(EndText),
    MoveByOffset(MoveByOffset),
    MoveByOffsetAndSetLeading(MoveByOffsetAndSetLeading),
    SetTextMatrix(SetTextMatrix),
    MoveToNextLine(MoveToNextLine),
    ShowText(ShowText),
    MoveToNextLineAndShowText(MoveToNextLineAndShowText),
    MoveToNextLineAndShowTextWithSpacing(MoveToNextLineAndShowTextWithSpacing),
    ShowTextArray(ShowTextArray),
    NotImplemented,
}

macro_rules! impl_from {
    ($($t:ident,)+) => {
        $(
            impl From<$t> for Operator {
                fn from(value: $t) -> Self {
                    Self::$t(value)
                }
            }
        )+
    };
}

impl_from!(
    SetCharacterSpacing,
    SetWordSpacing,
    SetHorizontalScaling,
    SetTextLeading,
    SetFontAndFontSize,
    SetTextRenderingMode,
    SetTextRise,
    BeginText,
    EndText,
    MoveByOffset,
    MoveByOffsetAndSetLeading,
    SetTextMatrix,
    MoveToNextLine,
    ShowText,
    MoveToNextLineAndShowText,
    MoveToNextLineAndShowTextWithSpacing,
    ShowTextArray,
);

#[derive(Error, Debug)]
pub enum OperatorError {
    #[error("invalid object")]
    InvalidObject,
    #[error("missing operand")]
    MissingOperand,
    #[error("eyre error")]
    Eyre,
}

impl From<OperatorError> for ErrMode<ContextError> {
    fn from(_: OperatorError) -> Self {
        ErrMode::Cut(ContextError::new())
    }
}

macro_rules! impl_try_from {
    ($($t:ty | $name:ident),+) => {
        $(
            impl TryFrom<Object> for $t {
                type Error = OperatorError;

                fn try_from(value: Object) -> Result<Self, Self::Error> {
                    if let Object::$name(inner) = value {
                        Ok(inner)
                    } else {
                        Err(OperatorError::InvalidObject)
                    }
                }
            }
        )+
    };
}

impl_try_from!(
    bool | Boolean,
    f32 | Real,
    i32 | Integer,
    Name | Name,
    Vec<u8> | String,
    Vec<Object> | Array
);

trait FromArgs: Sized {
    fn from_args(arguments: &mut Vec<Object>) -> Result<Self, OperatorError>;
}
