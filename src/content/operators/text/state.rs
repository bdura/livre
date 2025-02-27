use crate::extraction::Name;

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

/// `Tc` operator.
/// Set the caracter spacing, $T_c$, to a number expressed in unscaled text space units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SetCharacterSpacing(f32);

/// `Tw` operator.
/// Unscaled text space units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SetWordSpacing(f32);

/// `Tz` operator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SetHorizontalScaling(f32);

/// `TL` operator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SetTextLeading(f32);

/// `Tf` operator.
#[derive(Debug, Clone, PartialEq)]
pub struct SetFontAndFontSize(Name, f32);

/// `Tr` operator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SetTextRenderingMode(RenderingMode);

/// `Ts` operator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SetTextRise(f32);
