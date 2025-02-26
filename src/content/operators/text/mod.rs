//! Text operators, roughly organised following the PDF specification's hierarchy.

mod object;
mod positioning;
mod state;

pub use object::{BeginText, EndText};
pub use positioning::{
    MoveByOffset, MoveByOffsetAndSetLeading, MoveToNextLine, MoveToNextLineAndShowText,
    MoveToNextLineAndShowTextWithSpacing, SetTextMatrix, ShowText, ShowTextArray,
};
pub use state::{
    SetCharacterSpacing, SetFontAndFontSize, SetHorizontalScaling, SetTextLeading,
    SetTextRenderingMode, SetTextRise, SetWordSpacing,
};
