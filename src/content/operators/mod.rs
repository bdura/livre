mod accumulator;
mod text;

use text::{
    BeginText, EndText, MoveByOffset, MoveByOffsetAndSetLeading, MoveToNextLine,
    MoveToNextLineAndShowText, MoveToNextLineAndShowTextWithSpacing, SetCharacterSpacing,
    SetFontAndFontSize, SetHorizontalScaling, SetTextLeading, SetTextMatrix, SetTextRenderingMode,
    SetTextRise, SetWordSpacing, ShowText, ShowTextArray,
};

#[derive(Debug, Clone)]
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
