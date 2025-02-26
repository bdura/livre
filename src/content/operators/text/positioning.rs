use crate::extraction::LiteralString;

/// `Td` operator.
///
/// Move to the start of the next line, offset from the start of the current line
/// by `(tx, ty)`. `tx` and `ty` shall denote numbers expressed in unscaled
/// text space units
pub struct MoveByOffset {
    x: f32,
    y: f32,
}

/// `TD` operator.
///
/// Move to the start of the next line, offset from the start of the current line
/// by `(tx, ty)`. As a side effect, this operator shall set the leading parameter
/// in the text state. Equivalent to:
///
/// ```raw
/// -ty TL
/// tx ty Td
/// ```
pub struct MoveByOffsetAndSetLeading(f32, f32);

/// `Tm` operator.
pub struct SetTextMatrix(f32, f32, f32, f32, f32, f32);

/// `T*` operator
///
/// Move to the start of the next line. Equivalent to:
///
/// ```raw
/// 0 -T_l Td
/// ```
///
/// where `T_l` denotes the current leading parameter in the text state.
/// The negative of T l is used here because T l is the text leading expressed
/// as a positive number. Going to the next line entails decreasing the y coordinate.
pub struct MoveToNextLine;

/// `Tj` operator. Show a text string.
pub struct ShowText(LiteralString);

/// `'` operator. Equivalent to:
///
/// ```raw
/// T*
/// string Tj
/// ```
pub struct MoveToNextLineAndShowText(LiteralString);

/// `"` operator.
///
/// Move to the next line and show a text string, using `aw` as the word spacing and
/// `ac` as the character spacing (setting the corresponding parameters in the text state).
/// `aw` and `ac` shall be numbers expressed in unscaled text space units.
pub struct MoveToNextLineAndShowTextWithSpacing(f32, f32, LiteralString);

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
pub struct ShowTextArray; // TODO: add argument.
