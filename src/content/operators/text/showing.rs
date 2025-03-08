//! Text-showing operators. See section 9.4.3 of the PDF specification.

use std::fmt::Display;

use winnow::{combinator::peek, dispatch, token::any, BStr, PResult, Parser};

use crate::{
    content::{operators::behavior::TextOperation, state::TextObject},
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

impl Display for ShowText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// `'` operator. Equivalent to:
///
/// ```raw
/// T*
/// string Tj
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct MoveToNextLineAndShowText(PDFString);

extract_tuple!(MoveToNextLineAndShowText: 1);

impl Display for MoveToNextLineAndShowText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// `"` operator.
///
/// Move to the next line and show a text string, using `aw` as the word spacing and
/// `ac` as the character spacing (setting the corresponding parameters in the text state).
/// `aw` and `ac` shall be numbers expressed in unscaled text space units.
#[derive(Debug, Clone, PartialEq)]
pub struct MoveToNextLineAndShowTextWithSpacing(f32, f32, PDFString);

extract_tuple!(MoveToNextLineAndShowTextWithSpacing: 3);

impl Display for MoveToNextLineAndShowTextWithSpacing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

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

impl Display for ShowTextArray {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for element in &self.0 {
            if let TextArrayElement::Text(v) = element {
                write!(f, "{}", v)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TextArrayElement {
    Text(PDFString),
    Offset(f32),
}

impl From<PDFString> for TextArrayElement {
    fn from(value: PDFString) -> Self {
        Self::Text(value)
    }
}

impl From<&str> for TextArrayElement {
    fn from(value: &str) -> Self {
        Self::Text(value.into())
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

impl TextOperation for ShowText {
    fn apply(self, text_object: &mut TextObject) {
        text_object.add_text(self.0);
    }
}

impl TextOperation for MoveToNextLineAndShowText {
    fn apply(self, text_object: &mut TextObject) {
        text_object.move_to_next_line();
        text_object.add_text(self.0);
    }
}

impl TextOperation for MoveToNextLineAndShowTextWithSpacing {
    fn apply(self, text_object: &mut TextObject) {
        let MoveToNextLineAndShowTextWithSpacing(aw, ac, text) = self;

        text_object.move_to_next_line();
        text_object.set_word_spacing(aw);
        text_object.set_character_spacing(ac);
        text_object.add_text(text);
    }
}

impl TextOperation for ShowTextArray {
    fn apply(self, text_object: &mut TextObject) {
        text_object.add_text_array(self.0);
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use super::TextArrayElement::*;
    use super::*;

    use indoc::indoc;
    use rstest::rstest;

    #[rstest]
    #[case(
        indoc!{br#"
            [ (&''!\(\)) 7 (*+) -4 (,) -8 (-) 6 (!\(.) 3 (-) -7 (.\(/) 3 ] TJ
        "#},
        ShowTextArray(vec![
            "&''!()".into(),
            Offset(7.0),
            "*+".into(),
            Offset(-4.0),
            ",".into(),
            Offset(-8.0),
            "-".into(),
            Offset(6.0),
            "!(.".into(),
            Offset(3.0),
            "-".into(),
            Offset(-7.0),
            ".(/".into(),
            Offset(3.0),
        ])
    )]
    fn extraction<'de, T>(#[case] input: &'de [u8], #[case] expected: T)
    where
        T: Extract<'de> + Debug + PartialEq,
    {
        let result = extract(&mut input.as_ref()).unwrap();
        assert_eq!(expected, result);
    }
}
