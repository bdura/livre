use enum_dispatch::enum_dispatch;

use crate::{
    content::state::{RenderingMode, TextMatrix, TextStateParameters},
    extraction::{Extract, Name},
};

use super::PreTextOperation;

#[derive(Debug, Clone, PartialEq)]
#[enum_dispatch(PreTextOperation)]
pub enum TextStateOperator {
    SetCharacterSpacing(SetCharacterSpacing),
    SetWordSpacing(SetWordSpacing),
    SetHorizontalScaling(SetHorizontalScaling),
    SetTextLeading(SetTextLeading),
    SetFontAndFontSize(SetFontAndFontSize),
    SetTextRenderingMode(SetTextRenderingMode),
    SetTextRise(SetTextRise),
}

/// `Tc` operator.
/// Set the caracter spacing, $T_c$, to a number expressed in unscaled text space units.
///
/// ```raw
/// -0.024 Tc
/// 0.03 Tc
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Extract)]
pub struct SetCharacterSpacing(pub(crate) f32);

impl PreTextOperation for SetCharacterSpacing {
    fn preapply(self, _: &mut TextMatrix, parameters: &mut TextStateParameters) {
        parameters.character_spacing = self.0;
    }
}

/// `Tw` operator.
/// Unscaled text space units.
#[derive(Debug, Clone, Copy, PartialEq, Extract)]
pub struct SetWordSpacing(pub(crate) f32);

impl PreTextOperation for SetWordSpacing {
    fn preapply(self, _: &mut TextMatrix, parameters: &mut TextStateParameters) {
        parameters.word_spacing = self.0;
    }
}

/// `Tz` operator.
///
/// Adjusts the width of glyphs by stretching or compressing them in the horizontal direction.
/// Its value is the normalized value of the operand to the Tz operator which shall be specified
/// as a percentage of the normal width of the glyphs, with 100 being the normal width of 100%,
/// representing a scaling value of 1.0 for $T_h$.
///
/// ```raw
/// 100 Tz
/// 50 Tz
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Extract)]
pub struct SetHorizontalScaling(f32);

impl PreTextOperation for SetHorizontalScaling {
    fn preapply(self, _: &mut TextMatrix, parameters: &mut TextStateParameters) {
        parameters.horizontal_scaling = self.0 / 100.0;
    }
}

/// `TL` operator.
#[derive(Debug, Clone, Copy, PartialEq, Extract)]
pub struct SetTextLeading(f32);

impl PreTextOperation for SetTextLeading {
    fn preapply(self, _: &mut TextMatrix, parameters: &mut TextStateParameters) {
        parameters.leading = self.0;
    }
}

/// `Tf` operator.
///
/// ```raw
/// /F6 9 Tf
/// /F4 14.666667 Tf
/// ```
#[derive(Debug, Clone, PartialEq, Extract)]
pub struct SetFontAndFontSize(pub(crate) Name, pub(crate) f32);

impl PreTextOperation for SetFontAndFontSize {
    fn preapply(self, _matrix: &mut TextMatrix, _parameters: &mut TextStateParameters) {
        unreachable!("This operator is special-cased during initialisation of the TextObject, and cannot be applied twice.")
    }
}

/// `Tr` operator.
///
/// ```raw
/// 2 Tr
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Extract)]
pub struct SetTextRenderingMode(RenderingMode);

impl PreTextOperation for SetTextRenderingMode {
    fn preapply(self, _: &mut TextMatrix, parameters: &mut TextStateParameters) {
        parameters.rendering_mode = self.0;
    }
}

/// `Ts` operator.
#[derive(Debug, Clone, Copy, PartialEq, Extract)]
pub struct SetTextRise(f32);

impl PreTextOperation for SetTextRise {
    fn preapply(self, _: &mut TextMatrix, parameters: &mut TextStateParameters) {
        parameters.rise = self.0;
    }
}
