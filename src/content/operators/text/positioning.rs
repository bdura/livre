use enum_dispatch::enum_dispatch;

use crate::{
    content::state::TextStateParameters,
    debug, extract_tuple,
    extraction::{extract, Extract},
};

use super::PreTextOperation;

#[derive(Debug, Clone, PartialEq)]
#[enum_dispatch(PreTextOperation)]
pub enum TextPositioningOperator {
    MoveByOffset(MoveByOffset),
    MoveByOffsetAndSetLeading(MoveByOffsetAndSetLeading),
    SetTextMatrix(SetTextMatrix),
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
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveByOffset(f32, f32);

extract_tuple!(MoveByOffset: 2);

impl PreTextOperation for MoveByOffset {
    fn preapply(self, position: &mut (f32, f32), parameters: &mut TextStateParameters) {
        *position = (self.0, self.1);
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
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveByOffsetAndSetLeading(pub(crate) f32, pub(crate) f32);

extract_tuple!(MoveByOffsetAndSetLeading: 2);

impl PreTextOperation for MoveByOffsetAndSetLeading {
    fn preapply(self, position: &mut (f32, f32), parameters: &mut TextStateParameters) {
        parameters.leading = -self.1;
        *position = (self.0, self.1);
    }
}

/// `Tm` operator.
///
/// ```raw
/// 1 0 0 -1 370.70721 .47981739 Tm
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SetTextMatrix(f32, f32, f32, f32, f32, f32);

extract_tuple!(SetTextMatrix: 6);

impl PreTextOperation for SetTextMatrix {
    fn preapply(self, position: &mut (f32, f32), _: &mut TextStateParameters) {
        *position = (self.4, self.5);

        if self.0 != 1.0 || self.1 != 0.0 || self.2 != 0.0 || self.3 != 1.0 {
            debug!(
                "WARNING: non-trivial text matrix {:?}. Output will erroneous.",
                self
            );
        }
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
/// The negative of T l is used here because T l is the text leading expressed
/// as a positive number. Going to the next line entails decreasing the y coordinate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveToNextLine;

extract_tuple!(MoveToNextLine: 0);

impl PreTextOperation for MoveToNextLine {
    fn preapply(self, position: &mut (f32, f32), parameters: &mut TextStateParameters) {
        position.1 -= parameters.leading;
    }
}
