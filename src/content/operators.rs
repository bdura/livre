use crate::extraction::{LiteralString, Name};

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

// TODO: create dedicated types for each operator, for better behaviour locality.
pub enum Operator {
    /// `Tc` operator.
    /// Set the caracter spacing, $T_c$, to a number expressed in unscaled text space units.
    SetCharacterSpacing(f32),
    /// `Tw` operator.
    /// Unscaled text space units.
    SetWordSpacing(f32),
    /// `Tz` operator.
    SetHorizontalScaling(f32),
    /// `TL` operator.
    SetTextLeading(f32),
    /// `Tf` operator.
    SetFontAndFontSize(Name, f32),
    /// `Tr` operator.
    SetTextRenderingMode(RenderingMode),
    /// `Ts` operator.
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
    /// `Td` operator.
    ///
    /// Move to the start of the next line, offset from the start of the current line
    /// by `(tx, ty)`. `tx` and `ty` shall denote numbers expressed in unscaled
    /// text space units
    MoveByOffset(f32, f32),
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
    MoveByOffsetAndSetLeading(f32, f32),
    /// `Tm` operator.
    SetTextMatrix(f32, f32, f32, f32, f32, f32),
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
    MoveToNextLine,
    /// `Tj` operator. Show a text string.
    ShowText(LiteralString),
    /// `'` operator. Equivalent to:
    ///
    /// ```raw
    /// T*
    /// string Tj
    /// ```
    MoveToNextLineAndShowText(LiteralString),
    /// `"` operator.
    ///
    /// Move to the next line and show a text string, using `aw` as the word spacing and
    /// `ac` as the character spacing (setting the corresponding parameters in the text state).
    /// `aw` and `ac` shall be numbers expressed in unscaled text space units.
    MoveToNextLineAndShowTextWithSpacing(f32, f32, LiteralString),
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
    ShowTextArray, // TODO: add argument.
}
