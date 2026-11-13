//! Definition of a [text object](TextObject), and its state parameters.
//!
//! > At the beginning of a text object, $T_m$ shall be the identity matrix;
//! > therefore, the origin of text space shall be initially the same as that of
//! > user space. The text-positioning operators, described in
//! > "Table 106 — Text-positioning operators" alter $T_m$ and thereby control the
//! > placement of glyphs that are subsequently painted. Also, the text-showing
//! > operators, described in "Table 107 — Text-showing operators", update $T_m$
//! > (by altering its e and f translation components) to take into account
//! > the horizontal or vertical displacement of each glyph painted as well as
//! > any character or word-spacing parameters in the text state.

use std::collections::VecDeque;

use winnow::{
    combinator::trace,
    error::{ContextError, ErrMode},
    BStr, ModalResult, Parser,
};

use crate::{
    content::{
        error::{ContentError, Result},
        operators::{
            text::{TextArrayElement, TextOperation, TextOperator},
            Operator,
        },
    },
    extraction::{extract, Extract, Name, PDFString},
};

/// The font selected by the `Tf` operator: a resource name and a size.
#[derive(Debug, Clone, PartialEq)]
pub struct Font {
    /// Font resource name.
    /// NOTE: this is set to become an actual object in the future.
    pub name: Name,
    /// Font size, a scaling factor applied to every glyph's size parameters.
    pub size: f32,
}

/// Text state parameters, as defined in section 9.3 of the PDF specification.
#[derive(Debug, Clone, PartialEq)]
pub struct TextStateParameters {
    /// Font and font size. `None` until a `Tf` operator selects one.
    pub font: Option<Font>,
    /// Spacing between characters, in unscaled text space units. Added to the horizontal or
    /// vertical component of the glyph's displacement, depending on the writing mode.
    ///
    /// The actual spacing may be subject to the [horizontal scaling](Self::horizontal_scaling)
    /// should the writing mode be horizontal.
    ///
    /// Note that since the origin is located in the lower-left corner of the glyph, positive
    /// values move the glyph **up** in vertical writing mode..
    pub character_spacing: f32,
    /// Spacing between words, in unscaled text space units. Works the same way as
    /// [character spacing](Self::character_spacing).
    ///
    /// From the specification:
    ///
    /// > Word spacing shall be applied to every occurrence of the single-byte character code 32
    /// > in a string when using a simple font (including Type 3) or a composite font that defines
    /// > code 32 as a single-byte code. It shall not apply to occurrences of the byte value 32
    /// > in multiple-byte codes
    pub word_spacing: f32,
    /// Horizontal scaling: adjusts the width of glyphs by stretching or compressing them in the
    /// horizontal direction. Specified as a percentage of the normal width of the glyph, 100
    /// being the normal width.
    pub horizontal_scaling: f32,
    /// Vertical distance between the baselines of two consecutive lines of text, in unscaled text
    /// units.
    pub leading: f32,
    /// Text rendering mode.
    pub rendering_mode: RenderingMode,
    /// Distance to move the baseline up or down from its default location. Contrary to character
    /// spacing, positive values move the baseline up, negative values move it down.
    pub rise: f32,
}

impl Default for TextStateParameters {
    fn default() -> Self {
        Self {
            font: None,
            character_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            leading: 0.0,
            rendering_mode: RenderingMode::Fill,
            rise: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextMatrix {
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
    f: f32,
}

impl Extract<'_> for TextMatrix {
    fn extract(input: &mut &BStr) -> ModalResult<Self> {
        trace(
            "livre-text-matrix",
            extract.map(|(a, b, c, d, e, f)| Self { a, b, c, d, e, f }),
        )
        .parse_next(input)
    }
}

impl Default for TextMatrix {
    fn default() -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        }
    }
}

impl TextMatrix {
    pub fn position(&self) -> (f32, f32) {
        (self.e, self.f)
    }

    pub fn move_to(&mut self, x: f32, y: f32) {
        let Self { a, b, c, d, e, f } = *self;

        let e = a * x + c * y + e;
        let f = b * x + d * y + f;

        *self = Self { a, b, c, d, e, f };
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
    fn extract(input: &mut &'_ winnow::BStr) -> winnow::ModalResult<Self> {
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

/// A text object.
///
/// Can be iterated over to extract text elements. Text is decoded
/// from raw [`PDFString`] bytes using best-effort heuristics
/// (UTF-16BE if a BOM is present, Latin-1 otherwise).
///
/// The default text object is the state at `BT`: identity matrix, default parameters and
/// no font.
#[derive(Default)]
pub struct TextObject {
    /// Text matrix. Governs the transformation of text space to user space.
    pub matrix: TextMatrix,
    /// All text state parameters: font, leading, character spacing, word spacing, etc.
    pub parameters: TextStateParameters,
    // FIXME: this indirection will be needed down the line. For now it seems a bit dumb.
    // It should be replaced with a `VecDeque<u8>` to allow the *font* to iterate over the text
    pub text_buffer: Option<PDFString>,
    /// Buffer of text elements and offsets. We use a [`VecDeque`] to allow for efficient popping.
    pub buffer: Option<VecDeque<TextArrayElement>>,
}

impl TextObject {
    pub fn set_font(&mut self, font: Font) {
        self.parameters.font = Some(font);
    }
    pub fn move_to(&mut self, x: f32, y: f32) {
        self.matrix.move_to(x, y);
    }
    pub fn move_to_next_line(&mut self) {
        self.matrix.move_to(0.0, -self.parameters.leading);
    }
    pub fn add_text(&mut self, text: PDFString) {
        self.text_buffer = Some(text);
    }
    pub fn add_text_array(&mut self, array: Vec<TextArrayElement>) {
        self.buffer = Some(array.into());
    }
    pub fn set_leading(&mut self, leading: f32) {
        self.parameters.leading = leading;
    }
    pub fn set_character_spacing(&mut self, ac: f32) {
        self.parameters.character_spacing = ac;
    }
    pub fn set_word_spacing(&mut self, aw: f32) {
        self.parameters.word_spacing = aw;
    }
    pub fn set_horizontal_scaling(&mut self, scaling: f32) {
        self.parameters.horizontal_scaling = scaling;
    }
}

/// The `TextObjectStream` holds a text object and a stream of operators that apply to it.
pub struct TextObjectStream<Ops> {
    text_object: TextObject,
    ops: Ops,
}

impl<Ops> TextObjectStream<Ops> {
    /// The text object in its current state: the font, matrix and parameters that applied
    /// to the last element yielded by the stream.
    pub fn text_object(&self) -> &TextObject {
        &self.text_object
    }
}

impl TextObject {
    /// Apply the operators following a `BT` to a fresh text object until a font is selected.
    ///
    /// Returns `None` if the text object is closed by `ET` before any text could be shown:
    /// it is empty, and there is nothing to iterate over.
    fn build<Ops>(ops: &mut Ops) -> Result<Option<Self>>
    where
        Ops: Iterator<Item = Operator>,
    {
        let mut text_object = TextObject::default();

        for operator in ops {
            match operator {
                Operator::EndText(_) => return Ok(None),
                Operator::Text(TextOperator::TextShowingOperator(op)) => {
                    return Err(ContentError::UnexpectedTextShowingOperator(op));
                }
                Operator::Text(op) => {
                    op.apply(&mut text_object);

                    if text_object.parameters.font.is_some() {
                        return Ok(Some(text_object));
                    }
                }
                _ => {
                    tracing::debug!("Skipping operator: {:?}", operator);
                }
            }
        }

        Err(ContentError::IncompleteTextObject)
    }
}

/// Main, if temporary, entry point for parsing text objects.
///
/// Skips over any operators until it finds the `BT` operator, which marks the beginning of a text,
/// and returns an object that can be used to iterate over the text elements. Empty text objects
/// (closed before any font is selected) are skipped.
pub fn parse_text_object<Ops>(mut ops: Ops) -> Result<Option<TextObjectStream<Ops>>>
where
    Ops: Iterator<Item = Operator>,
{
    loop {
        // NOTE: just skip any other operators until we find the text object
        if !ops.by_ref().any(|op| matches!(op, Operator::BeginText(_))) {
            return Ok(None);
        }

        if let Some(text_object) = TextObject::build(&mut ops)? {
            return Ok(Some(TextObjectStream { text_object, ops }));
        }
    }
}

impl Iterator for TextObject {
    // NOTE: text is decoded from raw PDF string bytes using best-effort heuristics
    // (UTF-16BE if a BOM is present, Latin-1 otherwise). Actual decoding requires
    // the font's encoding map.
    type Item = ((f32, f32), String);

    fn next(&mut self) -> Option<Self::Item> {
        if self.text_buffer.is_none() {
            let buffer = self.buffer.as_mut()?;

            loop {
                match buffer.pop_front()? {
                    TextArrayElement::Text(text) => {
                        self.text_buffer = Some(text);
                        break;
                    }
                    TextArrayElement::Offset(offset) => {
                        // NOTE: offset is given in thousandths of a unit of text space
                        self.matrix.move_to(-offset / 1_000.0, 0.0);
                    }
                }
            }
        }

        // NOTE: decoding is best-effort; see `PDFString::decode` for the heuristic.
        let text = self.text_buffer.take()?.decode();

        Some((self.matrix.position(), text))
    }
}

impl<Ops> Iterator for TextObjectStream<Ops>
where
    Ops: Iterator<Item = Operator>,
{
    type Item = ((f32, f32), String);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(out) = self.text_object.next() {
                return Some(out);
            }

            let op = self.ops.next()?;
            match op {
                Operator::EndText(_) => return None,
                Operator::Text(op) => op.apply(&mut self.text_object),
                _ => {
                    tracing::debug!("Skipping operator: {:?}", op);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use winnow::{ascii::multispace0, combinator::preceded};

    use super::*;

    /// Parse a content stream into its operators.
    fn operators(input: &[u8]) -> Vec<Operator> {
        let mut stream = BStr::new(input);
        let mut it =
            winnow::combinator::iterator(&mut stream, preceded(multispace0, Operator::extract));
        let ops: Vec<Operator> = (&mut it).collect();
        it.finish().unwrap();
        ops
    }

    fn font(name: &str, size: f32) -> Font {
        Font {
            name: Name(name.as_bytes().to_vec()),
            size,
        }
    }

    #[test]
    fn second_tf_updates_the_font() {
        let ops = operators(b"BT /F1 12 Tf (a) Tj /F2 10 Tf (b) Tj ET");
        let mut stream = parse_text_object(ops.into_iter()).unwrap().unwrap();

        assert_eq!(stream.text_object.parameters.font, Some(font("F1", 12.0)));

        let (_, text) = stream.next().unwrap();
        assert_eq!(text, "a");
        assert_eq!(stream.text_object.parameters.font, Some(font("F1", 12.0)));

        let (_, text) = stream.next().unwrap();
        assert_eq!(text, "b");
        assert_eq!(stream.text_object.parameters.font, Some(font("F2", 10.0)));

        assert!(stream.next().is_none());
    }

    #[test]
    fn empty_text_object_is_skipped() {
        // Without the skip, the `Td` of the empty object would leak into the next one.
        let ops = operators(b"BT 0 10 Td ET BT /F1 12 Tf (a) Tj ET");
        let mut stream = parse_text_object(ops.into_iter()).unwrap().unwrap();

        assert_eq!(stream.next(), Some(((0.0, 0.0), "a".to_string())));
        assert!(stream.next().is_none());
    }

    #[test]
    fn trailing_empty_text_object_ends_the_content() {
        let ops = operators(b"BT ET");
        assert!(parse_text_object(ops.into_iter()).unwrap().is_none());
    }

    #[test]
    fn showing_text_before_a_font_is_an_error() {
        let ops = operators(b"BT (a) Tj ET");
        assert!(matches!(
            parse_text_object(ops.into_iter()),
            Err(ContentError::UnexpectedTextShowingOperator(_))
        ));
    }
}
