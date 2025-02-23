use crate::extraction::Name;

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

pub enum Operator {
    /// Operator `Tc`.
    /// Set the caracter spacing, $T_c$, to a number expressed in unscaled text space units.
    SetCharacterSpacing(f32),
    /// Operator `Tw`
    /// Unscaled text space units.
    SetWordSpacing(f32),
    /// Operator `Tz`
    SetHorizontalScaling(f32),
    /// Operator `TL`.
    SetTextLeading(f32),
    /// Operator `Tf`
    SetFontAndFontSize(Name, f32),
    /// Operator `Tr`
    SetTextRenderingMode(RenderingMode),
    /// Operator `Ts`.
    SetTextRise(f32),
    /// `BT` operator.
    ///
    /// Begin a text object, initializing the text matrix $T_m$ and the text line matrix
    /// $T_{lm}$, to the identity matrix. Text objects shall not be nested; a second
    /// `BeginText` shall not apear before an [`EndText`](Self::EndText).
    ///
    BeginText,
    /// `ET` operator.
    ///
    /// End a text object, discarding the text matrix.
    EndText,
}
