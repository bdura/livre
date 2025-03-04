use winnow::{combinator::peek, dispatch, token::any, BStr, PResult, Parser};

use crate::{
    extract_tuple,
    extraction::{extract, Extract, HexadecimalString, LiteralString, PDFString},
};

/// `Tj` operator. Show a text string.
///
/// ```raw
/// <0052> Tj
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ShowText(PDFString);

extract_tuple!(ShowText: 1);

/// `'` operator. Equivalent to:
///
/// ```raw
/// T*
/// string Tj
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct MoveToNextLineAndShowText(PDFString);

extract_tuple!(MoveToNextLineAndShowText: 1);

/// `"` operator.
///
/// Move to the next line and show a text string, using `aw` as the word spacing and
/// `ac` as the character spacing (setting the corresponding parameters in the text state).
/// `aw` and `ac` shall be numbers expressed in unscaled text space units.
#[derive(Debug, Clone, PartialEq)]
pub struct MoveToNextLineAndShowTextWithSpacing(f32, f32, PDFString);

extract_tuple!(MoveToNextLineAndShowTextWithSpacing: 3);

/// `TJ` operator.
///
/// Show zero or more text strings, allowing individual glyph positioning.
/// Each element of the array is either a string or a number:
///
/// - in the case of a string, the operator shows the text;
/// - in the case of a number, the operator adjust the text position by that
///   amount (i.e. translate the text matrix). Expressed in **thousandths of a unit of
///   text space.
///   That amount is substracted from the current "selected coordinate",
///   depending on the writing mode.
///
/// ```raw
/// [(5)-6(1)-6(,)-2( )-2(A)] TJ
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ShowTextArray(Vec<TextArrayElement>);

extract_tuple!(ShowTextArray: 1);

#[derive(Debug, Clone, PartialEq)]
enum TextArrayElement {
    Text(PDFString),
    Offset(f32),
}

impl From<PDFString> for TextArrayElement {
    fn from(value: PDFString) -> Self {
        Self::Text(value)
    }
}

impl From<HexadecimalString> for TextArrayElement {
    fn from(value: HexadecimalString) -> Self {
        Self::Text(value.into())
    }
}

impl From<LiteralString> for TextArrayElement {
    fn from(value: LiteralString) -> Self {
        Self::Text(value.into())
    }
}

impl From<f32> for TextArrayElement {
    fn from(value: f32) -> Self {
        Self::Offset(value)
    }
}

impl Extract<'_> for TextArrayElement {
    fn extract(input: &mut &BStr) -> PResult<Self> {
        dispatch! {peek(any);
            b'(' => LiteralString::extract.map(TextArrayElement::from),
            b'<' => HexadecimalString::extract.map(TextArrayElement::from),
            _ => extract.map(TextArrayElement::Offset),
        }
        .parse_next(input)
    }
}
