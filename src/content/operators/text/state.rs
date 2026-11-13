use enum_dispatch::enum_dispatch;

use crate::{
    content::state::{Font, RenderingMode, TextObject},
    extraction::{Extract, Name},
};

use super::TextOperation;

#[derive(Debug, Clone, PartialEq)]
#[enum_dispatch(TextOperation)]
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

impl TextOperation for SetCharacterSpacing {
    fn apply(self, text_object: &mut TextObject) {
        text_object.set_character_spacing(self.0);
    }
}

/// `Tw` operator.
/// Unscaled text space units.
#[derive(Debug, Clone, Copy, PartialEq, Extract)]
pub struct SetWordSpacing(pub(crate) f32);

impl TextOperation for SetWordSpacing {
    fn apply(self, text_object: &mut TextObject) {
        text_object.set_word_spacing(self.0);
    }
}

/// `Tz` operator.
#[derive(Debug, Clone, Copy, PartialEq, Extract)]
pub struct SetHorizontalScaling(f32);

impl TextOperation for SetHorizontalScaling {
    fn apply(self, text_object: &mut TextObject) {
        text_object.set_horizontal_scaling(self.0);
    }
}

/// `TL` operator.
#[derive(Debug, Clone, Copy, PartialEq, Extract)]
pub struct SetTextLeading(f32);

impl TextOperation for SetTextLeading {
    fn apply(self, text_object: &mut TextObject) {
        text_object.set_leading(self.0);
    }
}

/// `Tf` operator.
///
/// Selects the font and font size. May appear any number of times within a text object;
/// each occurrence replaces the previous selection.
///
/// ```raw
/// /F6 9 Tf
/// /F4 14.666667 Tf
/// ```
#[derive(Debug, Clone, PartialEq, Extract)]
pub struct SetFontAndFontSize(pub(crate) Name, pub(crate) f32);

impl TextOperation for SetFontAndFontSize {
    fn apply(self, text_object: &mut TextObject) {
        let SetFontAndFontSize(name, size) = self;
        text_object.set_font(Font { name, size });
    }
}

/// `Tr` operator.
///
/// ```raw
/// 2 Tr
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Extract)]
pub struct SetTextRenderingMode(RenderingMode);

impl TextOperation for SetTextRenderingMode {
    fn apply(self, text_object: &mut TextObject) {
        text_object.parameters.rendering_mode = self.0;
    }
}

/// `Ts` operator.
#[derive(Debug, Clone, Copy, PartialEq, Extract)]
pub struct SetTextRise(f32);

impl TextOperation for SetTextRise {
    fn apply(self, text_object: &mut TextObject) {
        text_object.parameters.rise = self.0;
    }
}
