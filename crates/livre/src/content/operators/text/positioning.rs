use enum_dispatch::enum_dispatch;

use crate::{
    content::state::{Matrix, TextObject},
    extraction::Extract,
};

use super::TextOperation;

/// Abstraction over text positioning operators, as defined in section 9.4.2 of the PDF
/// specification.
#[derive(Debug, Clone, PartialEq)]
#[enum_dispatch(TextOperation)]
pub enum TextPositioningOperator {
    /// The `Td` operator. Move to the start of the next line, offset from the start of the current
    /// line by `(tx, ty)`.
    MoveByOffset(MoveByOffset),
    /// The `TD` operator. Move to the start of the next line, and set the leading parameter.
    MoveByOffsetAndSetLeading(MoveByOffsetAndSetLeading),
    /// The `Tm` operator. Set the text matrix.
    SetTextMatrix(SetTextMatrix),
    /// The `T*` operator. Move to the start of the next line.
    MoveToNextLine(MoveToNextLine),
}

/// `Td` operator.
///
/// Move to the start of the next line, offset from the start of the current line
/// by `(tx, ty)`. `tx` and `ty` shall denote numbers expressed in unscaled
/// text space units
///
/// ```raw
/// 0 -13.2773438 Td
/// 8.1511078 0 Td
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Extract)]
pub struct MoveByOffset(f32, f32);

impl TextOperation for MoveByOffset {
    fn apply(self, text_object: &mut TextObject) {
        text_object.move_to(self.0, self.1);
    }
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
#[derive(Debug, Clone, Copy, PartialEq, Extract)]
pub struct MoveByOffsetAndSetLeading(pub(crate) f32, pub(crate) f32);

impl TextOperation for MoveByOffsetAndSetLeading {
    fn apply(self, text_object: &mut TextObject) {
        text_object.set_leading(-self.1);
        text_object.move_to(self.0, self.1);
    }
}

/// `Tm` operator.
///
/// ```raw
/// 1 0 0 -1 370.70721 .47981739 Tm
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Extract)]
pub struct SetTextMatrix(Matrix);

impl TextOperation for SetTextMatrix {
    fn apply(self, text_object: &mut TextObject) {
        text_object.matrix = self.0;
    }
}

/// `T*` operator
///
/// Move to the start of the next line. Equivalent to:
///
/// ```raw
/// 0 -T_l Td
/// ```
///
/// where `T_l` denotes the current leading parameter in the text state.
/// The negative of $T_l$ is used here because $T_l$ is the text leading expressed
/// as a positive number. Going to the next line entails decreasing the y coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Extract)]
pub struct MoveToNextLine;

impl TextOperation for MoveToNextLine {
    fn apply(self, text_object: &mut TextObject) {
        text_object.move_to_next_line();
    }
}
