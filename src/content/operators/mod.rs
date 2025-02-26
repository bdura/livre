mod text;

use text::{
    BeginText, EndText, MoveByOffset, MoveByOffsetAndSetLeading, MoveToNextLine,
    MoveToNextLineAndShowText, MoveToNextLineAndShowTextWithSpacing, SetCharacterSpacing,
    SetFontAndFontSize, SetHorizontalScaling, SetTextLeading, SetTextMatrix, SetTextRenderingMode,
    SetTextRise, SetWordSpacing, ShowText, ShowTextArray,
};

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
}
