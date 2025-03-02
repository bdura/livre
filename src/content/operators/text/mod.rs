//! Text operators, roughly organised following the PDF specification's hierarchy.

mod object;
mod positioning;
mod showing;
mod state;

pub use object::{BeginText, EndText};
pub use positioning::{MoveByOffset, MoveByOffsetAndSetLeading, MoveToNextLine, SetTextMatrix};
pub use showing::{
    MoveToNextLineAndShowText, MoveToNextLineAndShowTextWithSpacing, ShowText, ShowTextArray,
};
pub use state::{
    SetCharacterSpacing, SetFontAndFontSize, SetHorizontalScaling, SetTextLeading,
    SetTextRenderingMode, SetTextRise, SetWordSpacing,
};
