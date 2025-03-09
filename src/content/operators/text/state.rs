use enum_dispatch::enum_dispatch;
use winnow::{
    error::{ContextError, ErrMode},
    Parser,
};

use crate::{
    content::state::{TextMatrix, TextStateParameters},
    extract_tuple,
    extraction::{extract, Extract, Name},
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
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SetCharacterSpacing(pub(crate) f32);

extract_tuple!(SetCharacterSpacing: 1);

impl PreTextOperation for SetCharacterSpacing {
    fn preapply(self, _: &mut TextMatrix, parameters: &mut TextStateParameters) {
        parameters.character_spacing = self.0;
    }
}

/// `Tw` operator.
/// Unscaled text space units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SetWordSpacing(pub(crate) f32);
extract_tuple!(SetWordSpacing: 1);
impl PreTextOperation for SetWordSpacing {
    fn preapply(self, _: &mut TextMatrix, parameters: &mut TextStateParameters) {
        parameters.word_spacing = self.0;
    }
}

/// `Tz` operator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SetHorizontalScaling(pub(crate) f32);
extract_tuple!(SetHorizontalScaling: 1);
impl PreTextOperation for SetHorizontalScaling {
    fn preapply(self, _: &mut TextMatrix, parameters: &mut TextStateParameters) {
        parameters.horizontal_scaling = self.0;
    }
}

/// `TL` operator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SetTextLeading(pub(crate) f32);
extract_tuple!(SetTextLeading: 1);
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
#[derive(Debug, Clone, PartialEq)]
pub struct SetFontAndFontSize(pub(crate) Name, pub(crate) f32);
extract_tuple!(SetFontAndFontSize: 2);
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
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SetTextRenderingMode(pub(crate) RenderingMode);
extract_tuple!(SetTextRenderingMode: 1);
impl PreTextOperation for SetTextRenderingMode {
    fn preapply(self, _: &mut TextMatrix, parameters: &mut TextStateParameters) {
        parameters.rendering_mode = self.0;
    }
}

/// `Ts` operator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SetTextRise(pub(crate) f32);
extract_tuple!(SetTextRise: 1);
impl PreTextOperation for SetTextRise {
    fn preapply(self, _: &mut TextMatrix, parameters: &mut TextStateParameters) {
        parameters.rise = self.0;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RenderingMode {
    /// Fill text.
    Fill,
    /// Stroke text.
    Stroke,
    /// Fill, then stroke text.
    FillThenStroke,
    /// Neither fill nor stroke text (invisible).
    Invisible,
    /// Fill text and add to path for clipping.
    FillAndClip,
    /// Stroke text and add to path for clipping.
    StrokeAndClip,
    /// Fill, then stroke text and add to path for clipping.
    FillThenStrokeAndClip,
    /// Add text to path for clipping.
    AddTextAndClip,
}

impl Extract<'_> for RenderingMode {
    fn extract(input: &mut &'_ winnow::BStr) -> winnow::PResult<Self> {
        match u8::extract(input)? {
            0 => Ok(Self::Fill),
            1 => Ok(Self::Stroke),
            2 => Ok(Self::FillThenStroke),
            3 => Ok(Self::Invisible),
            4 => Ok(Self::FillAndClip),
            5 => Ok(Self::StrokeAndClip),
            6 => Ok(Self::FillThenStrokeAndClip),
            7 => Ok(Self::AddTextAndClip),
            _ => Err(ErrMode::Backtrack(ContextError::new())),
        }
    }
}
