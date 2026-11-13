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
    error::{ContextError, ErrMode},
    BStr, ModalResult,
};

use crate::{
    content::{
        error::{ContentError, Result},
        operators::{
            graphics::GraphicsStateOperation,
            text::{TextArrayElement, TextOperation, TextOperator},
            Operator,
        },
        state::{GraphicsState, Matrix},
    },
    extraction::{Extract, Name, PDFString},
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
    fn extract(input: &mut &'_ BStr) -> ModalResult<Self> {
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
/// The default text object is the state at `BT` on a page without `cm`: identity text
/// matrix and CTM, default parameters and no font.
#[derive(Default)]
pub struct TextObject {
    /// Text matrix. Governs the transformation of text space to user space.
    pub matrix: Matrix,
    /// The current transformation matrix in force at `BT`. Governs the transformation of
    /// user space to default user space. Constant for the whole text object, since `cm`,
    /// `q` and `Q` are not allowed inside one.
    pub ctm: Matrix,
    /// All text state parameters: font, leading, character spacing, word spacing, etc.
    pub parameters: TextStateParameters,
    // FIXME: this indirection will be needed down the line. For now it seems a bit dumb.
    // It should be replaced with a `VecDeque<u8>` to allow the *font* to iterate over the text
    pub text_buffer: Option<PDFString>,
    /// Buffer of text elements and offsets. We use a [`VecDeque`] to allow for efficient popping.
    pub buffer: Option<VecDeque<TextArrayElement>>,
}

impl TextObject {
    /// The matrix mapping text space to default user space: `Tm × CTM`. Its translation is
    /// the position the iterator yields; its other components carry the orientation and
    /// scale of the text, before the font size and rise are applied.
    pub fn transformation(&self) -> Matrix {
        self.matrix.then(self.ctm)
    }

    pub fn set_font(&mut self, font: Font) {
        self.parameters.font = Some(font);
    }
    pub fn move_to(&mut self, x: f32, y: f32) {
        self.matrix.translate(x, y);
    }
    pub fn move_to_next_line(&mut self) {
        self.matrix.translate(0.0, -self.parameters.leading);
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
    fn build<Ops>(ops: &mut Ops, ctm: Matrix) -> Result<Option<Self>>
    where
        Ops: Iterator<Item = Operator>,
    {
        let mut text_object = TextObject {
            ctm,
            ..TextObject::default()
        };

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
///
/// Graphics state operators met on the way (`q`, `Q`, `cm`) update `graphics_state`, and the
/// text object captures the CTM in force at `BT`. The same `graphics_state` must be passed to
/// every call on one content stream, since a `cm` at the top of the page applies to every
/// text object below it.
pub fn parse_text_object<Ops>(
    mut ops: Ops,
    graphics_state: &mut GraphicsState,
) -> Result<Option<TextObjectStream<Ops>>>
where
    Ops: Iterator<Item = Operator>,
{
    loop {
        match ops.next() {
            None => return Ok(None),
            Some(Operator::BeginText(_)) => {
                if let Some(text_object) = TextObject::build(&mut ops, graphics_state.ctm())? {
                    return Ok(Some(TextObjectStream { text_object, ops }));
                }
            }
            Some(Operator::GraphicsState(op)) => op.apply(graphics_state),
            Some(_) => {}
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
                        self.matrix.translate(-offset / 1_000.0, 0.0);
                    }
                }
            }
        }

        // NOTE: decoding is best-effort; see `PDFString::decode` for the heuristic.
        let text = self.text_buffer.take()?.decode();

        Some((self.ctm.apply(self.matrix.translation()), text))
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
    use winnow::{ascii::multispace0, combinator::preceded, BStr};

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
        let mut stream = parse_text_object(ops.into_iter(), &mut GraphicsState::default())
            .unwrap()
            .unwrap();

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
        let mut stream = parse_text_object(ops.into_iter(), &mut GraphicsState::default())
            .unwrap()
            .unwrap();

        assert_eq!(stream.next(), Some(((0.0, 0.0), "a".to_string())));
        assert!(stream.next().is_none());
    }

    #[test]
    fn trailing_empty_text_object_ends_the_content() {
        let ops = operators(b"BT ET");
        assert!(
            parse_text_object(ops.into_iter(), &mut GraphicsState::default())
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn showing_text_before_a_font_is_an_error() {
        let ops = operators(b"BT (a) Tj ET");
        assert!(matches!(
            parse_text_object(ops.into_iter(), &mut GraphicsState::default()),
            Err(ContentError::UnexpectedTextShowingOperator(_))
        ));
    }
}

#[cfg(test)]
mod graphics_state_tests {
    use winnow::{ascii::multispace0, combinator::preceded, BStr};

    use super::*;

    fn operators(input: &[u8]) -> Vec<Operator> {
        let mut stream = BStr::new(input);
        let mut it =
            winnow::combinator::iterator(&mut stream, preceded(multispace0, Operator::extract));
        let ops: Vec<Operator> = (&mut it).collect();
        it.finish().unwrap();
        ops
    }

    /// Every run position of a content stream, with the graphics state carried across text
    /// objects as a caller would.
    fn positions(input: &[u8]) -> Vec<(f32, f32)> {
        let mut ops = operators(input).into_iter();
        let mut graphics_state = GraphicsState::default();
        let mut positions = Vec::new();

        while let Some(stream) = parse_text_object(&mut ops, &mut graphics_state).unwrap() {
            positions.extend(stream.map(|(position, _)| position));
        }

        positions
    }

    fn assert_close((x, y): (f32, f32), (ex, ey): (f32, f32)) {
        assert!(
            (x - ex).abs() < 1e-3 && (y - ey).abs() < 1e-3,
            "expected ({ex}, {ey}), got ({x}, {y})"
        );
    }

    #[test]
    fn cm_before_bt_transforms_the_run_position() {
        // (5, 7) in text space, scaled by 2 and moved by (10, 20).
        let positions = positions(b"2 0 0 2 10 20 cm BT /F1 12 Tf 5 7 Td (a) Tj ET");
        assert_eq!(positions, vec![(20.0, 34.0)]);
    }

    #[test]
    fn q_and_big_q_restore_the_ctm_between_text_objects() {
        let positions =
            positions(b"q 1 0 0 1 100 0 cm BT /F1 12 Tf (a) Tj ET Q BT /F1 12 Tf (b) Tj ET");
        assert_eq!(positions, vec![(100.0, 0.0), (0.0, 0.0)]);
    }

    #[test]
    fn unbalanced_big_q_is_ignored() {
        let positions = positions(b"Q 1 0 0 1 3 4 cm BT /F1 12 Tf (a) Tj ET");
        assert_eq!(positions, vec![(3.0, 4.0)]);
    }

    /// The first line of `tests/resources/text.pdf` (Skia output): the page is flipped once,
    /// then each line is wrapped in `q … Q` and positioned by its own `cm`. The expected
    /// value is `(0.75·x + 72, 792 − (0.75·(0.4798 + 13.2773) + 86.546))`.
    const SKIA_LINE: &[u8] = b"1 0 0 -1 0 792 cm
q
.75 0 0 .75 72 86.546265 cm
BT
/F4 14.666667 Tf
1 0 0 -1 80.65947 .47981739 Tm
0 -13.2773438 Td
<004D> Tj
ET
Q";

    #[test]
    fn skia_line_lands_in_the_text_column() {
        let positions = positions(SKIA_LINE);
        assert_eq!(positions.len(), 1);
        assert_close(positions[0], (132.4946, 695.1359));
    }

    #[test]
    fn transformation_is_the_text_matrix_times_the_ctm() {
        let mut ops = operators(SKIA_LINE).into_iter();
        let mut graphics_state = GraphicsState::default();
        let mut stream = parse_text_object(&mut ops, &mut graphics_state)
            .unwrap()
            .unwrap();
        stream.next().unwrap();

        let transformation = stream.text_object().transformation();

        // The `Tm` flip and the page flip cancel: text-space "up" is page "up", scaled by 0.75.
        assert_close((transformation.a, transformation.d), (0.75, 0.75));
        assert_close((transformation.b, transformation.c), (0.0, 0.0));
        assert_close(transformation.translation(), (132.4946, 695.1359));
    }
}
