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
            text::{TextArrayElement, TextOperator, TextShowingOperation, TextStateOperation},
            Operator,
        },
        state::{
            space::{DefaultUser, Displacement, Point, Text, User},
            GraphicsState, Matrix,
        },
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

impl TextStateParameters {
    /// Select the font and font size (`Tf`).
    pub fn set_font(&mut self, font: Font) {
        self.font = Some(font);
    }
    /// Set the character spacing, in unscaled text space units (`Tc`).
    pub fn set_character_spacing(&mut self, spacing: f32) {
        self.character_spacing = spacing;
    }
    /// Set the word spacing, in unscaled text space units (`Tw`).
    pub fn set_word_spacing(&mut self, spacing: f32) {
        self.word_spacing = spacing;
    }
    /// Set the horizontal scaling, as a percentage of the normal glyph width (`Tz`).
    pub fn set_horizontal_scaling(&mut self, scaling: f32) {
        self.horizontal_scaling = scaling;
    }
    /// Set the leading, in unscaled text space units (`TL`).
    pub fn set_leading(&mut self, leading: f32) {
        self.leading = leading;
    }
    /// Set the text rendering mode (`Tr`).
    pub fn set_rendering_mode(&mut self, mode: RenderingMode) {
        self.rendering_mode = mode;
    }
    /// Set the text rise, in unscaled text space units (`Ts`).
    pub fn set_rise(&mut self, rise: f32) {
        self.rise = rise;
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

/// A shown string, as yielded by a [`TextObjectStream`].
#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    /// The start of the shown string in default user space: the translation of the text
    /// matrix, mapped through the CTM in force at `BT`.
    pub position: Point<DefaultUser>,
    /// The string's bytes, undecoded: their meaning depends on the font's encoding.
    /// [`PDFString::decode`] is a best-effort heuristic (UTF-16BE behind a BOM, Latin-1
    /// otherwise) until fonts are modelled.
    pub text: PDFString,
}

/// A text object: the state the text operators act on between `BT` and `ET`.
///
/// Its shown strings are read through the [`TextObjectStream`] that owns it.
pub struct TextObject<'a> {
    /// Text matrix. Governs the transformation of text space to user space.
    pub matrix: Matrix<Text, User>,
    /// The current transformation matrix in force at `BT`. Governs the transformation of
    /// user space to default user space. Constant for the whole text object, since `cm`,
    /// `q` and `Q` are not allowed inside one.
    pub ctm: Matrix<User, DefaultUser>,
    /// All text state parameters: font, leading, character spacing, word spacing, etc.
    ///
    /// Borrowed from the graphics state: the text state operators inside the object mutate
    /// it in place, so a `Tf` here is still in force in the next text object.
    pub parameters: &'a mut TextStateParameters,
    // FIXME: this indirection will be needed down the line. For now it seems a bit dumb.
    // It should be replaced with a `VecDeque<u8>` to allow the *font* to iterate over the text
    pub text_buffer: Option<PDFString>,
    /// Buffer of text elements and offsets. We use a [`VecDeque`] to allow for efficient popping.
    pub buffer: Option<VecDeque<TextArrayElement>>,
}

impl<'a> TextObject<'a> {
    /// The text object at `BT`: identity text matrix, nothing to show yet.
    fn begin(ctm: Matrix<User, DefaultUser>, parameters: &'a mut TextStateParameters) -> Self {
        Self {
            matrix: Matrix::coincident(),
            ctm,
            parameters,
            text_buffer: None,
            buffer: None,
        }
    }

    /// The matrix mapping text space to default user space: `Tm × CTM`. Its translation is
    /// the position the iterator yields; its other components carry the orientation and
    /// scale of the text, before the font size and rise are applied.
    pub fn transformation(&self) -> Matrix<Text, DefaultUser> {
        self.matrix.then(self.ctm)
    }

    /// Translate the text matrix by `offset`, expressed in text space (`Td`).
    pub fn move_to(&mut self, offset: Displacement<Text>) {
        self.matrix.translate(offset);
    }
    /// Translate the text matrix down by the leading, to the start of the next line (`T*`).
    pub fn move_to_next_line(&mut self) {
        self.move_to(Displacement::new(0.0, -self.parameters.leading));
    }
    /// Buffer a shown string (`Tj`), to be yielded as the next run.
    pub fn add_text(&mut self, text: PDFString) {
        self.text_buffer = Some(text);
    }
    /// Buffer the strings and offsets of a `TJ` array, to be yielded as runs in order, each
    /// offset moving the text matrix before the string that follows it.
    pub fn add_text_array(&mut self, array: Vec<TextArrayElement>) {
        self.buffer = Some(array.into());
    }

    /// The next run buffered by the last showing operator, if any. Consumes the `TJ`
    /// offsets met before it, moving the text matrix.
    fn next_run(&mut self) -> Option<Run> {
        if self.text_buffer.is_none() {
            let buffer = self.buffer.as_mut()?;

            loop {
                match buffer.pop_front()? {
                    TextArrayElement::Text(text) => {
                        self.text_buffer = Some(text);
                        break;
                    }
                    TextArrayElement::Offset(offset) => {
                        // ISO 32000-2, 9.4.4: tx = -n/1000 · Tfs · Th, the offset being
                        // in thousandths of a unit of glyph space.
                        let size = self.parameters.font.as_ref().map_or(0.0, |f| f.size);
                        let scaling = self.parameters.horizontal_scaling / 100.0;
                        let shift: Displacement<Text> =
                            Displacement::new(-offset / 1_000.0 * size * scaling, 0.0);
                        self.matrix.translate(shift);
                    }
                }
            }
        }

        let text = self.text_buffer.take()?;
        Some(Run {
            position: self.ctm.apply(self.matrix.translation()),
            text,
        })
    }
}

/// The part of a [`TextObject`] that does not borrow the graphics state. The text object
/// is opened under a short borrow and rebuilt from this under the stream's, once it is
/// known to show text.
struct Started {
    matrix: Matrix<Text, User>,
    text_buffer: Option<PDFString>,
    buffer: Option<VecDeque<TextArrayElement>>,
}

/// The text objects of a content stream, in order.
///
/// Owns the operator iterator and the graphics state it builds up: `q`, `Q`, `cm` and the
/// text state operators (`Tf`, `Tc`, ...) met outside `BT` update it, and each text object
/// starts from the CTM and text state then in force. Text positioning and showing
/// operators outside `BT` are illegal and skipped with a warning.
///
/// Not an [`Iterator`], since each item borrows it:
///
/// ```
/// # use livre::{content::{operators::Operator, Run, TextObjects}, extraction::Extract};
/// # use winnow::{ascii::multispace0, combinator::{iterator, preceded}, BStr};
/// # let mut content = BStr::new(b"/F1 12 Tf BT 72 700 Td (Hello) Tj ET");
/// # let mut operators = iterator(&mut content, preceded(multispace0, Operator::extract));
/// let mut objects = TextObjects::new(&mut operators);
/// while let Some(stream) = objects.next_object()? {
///     for Run { position, text } in stream {
/// #       assert_eq!((position.x, position.y, text.decode()), (72.0, 700.0, "Hello".into()));
///         /* ... */
///     }
/// }
/// # Ok::<(), livre::content::ContentError>(())
/// ```
pub struct TextObjects<Ops> {
    ops: Ops,
    graphics_state: GraphicsState,
    /// Inside a text object whose `ET` has not been read yet: its stream was dropped before
    /// the end, or opening it failed.
    open: bool,
}

impl<Ops> TextObjects<Ops>
where
    Ops: Iterator<Item = Operator>,
{
    /// Start at the top of a content stream, with the default graphics state.
    pub fn new(ops: Ops) -> Self {
        Self {
            ops,
            graphics_state: GraphicsState::default(),
            open: false,
        }
    }

    /// The next text object that shows text, or `None` at the end of the content stream.
    /// Text objects closed by `ET` before showing anything are skipped.
    ///
    /// If the previous stream was dropped before its `ET`, the rest of its text object is
    /// skipped first, its text state operators still applied.
    ///
    /// # Errors
    ///
    /// - [`ContentError::NoFontSelected`] if the text object shows text while no `Tf`
    ///   selected a font, in it or earlier in the content stream.
    /// - [`ContentError::IncompleteTextObject`] if the content stream ends inside `BT`
    ///   before any text is shown, or after the previous text object without its `ET`.
    pub fn next_object(&mut self) -> Result<Option<TextObjectStream<'_, Ops>>> {
        if self.open {
            self.skip_to_end()?;
        }

        loop {
            match self.ops.next() {
                None => return Ok(None),
                Some(Operator::BeginText(_)) => {
                    self.open = true;
                    if let Some(Started {
                        matrix,
                        text_buffer,
                        buffer,
                    }) = self.start()?
                    {
                        let text_object = TextObject {
                            matrix,
                            text_buffer,
                            buffer,
                            ..TextObject::begin(
                                self.graphics_state.ctm(),
                                self.graphics_state.text_state_mut(),
                            )
                        };
                        return Ok(Some(TextObjectStream {
                            text_object,
                            ops: &mut self.ops,
                            open: &mut self.open,
                        }));
                    }
                }
                Some(Operator::GraphicsState(op)) => op.apply(&mut self.graphics_state),
                Some(Operator::Text(TextOperator::TextStateOperator(op))) => {
                    op.apply(self.graphics_state.text_state_mut())
                }
                Some(Operator::Text(op)) => {
                    tracing::warn!("Text operator outside `BT`: skipped: {:?}", op);
                }
                Some(Operator::EndText(_)) => {
                    tracing::warn!("`ET` without a matching `BT`: skipped");
                }
                Some(op) => {
                    tracing::debug!("Skipping operator: {:?}", op);
                }
            }
        }
    }

    /// Apply the operators following a `BT` until the first showing operator, and return
    /// the text object as it shows it. `None` if `ET` comes first: the object is empty.
    fn start(&mut self) -> Result<Option<Started>> {
        let mut text_object = TextObject::begin(
            self.graphics_state.ctm(),
            self.graphics_state.text_state_mut(),
        );

        for operator in &mut self.ops {
            match operator {
                Operator::EndText(_) => {
                    self.open = false;
                    return Ok(None);
                }
                Operator::Text(TextOperator::TextShowingOperator(op)) => {
                    if text_object.parameters.font.is_none() {
                        return Err(ContentError::NoFontSelected);
                    }
                    op.apply(&mut text_object);

                    let TextObject {
                        matrix,
                        text_buffer,
                        buffer,
                        ..
                    } = text_object;
                    return Ok(Some(Started {
                        matrix,
                        text_buffer,
                        buffer,
                    }));
                }
                Operator::Text(op) => op.apply(&mut text_object),
                _ => {
                    tracing::debug!("Skipping operator: {:?}", operator);
                }
            }
        }

        self.open = false;
        Err(ContentError::IncompleteTextObject)
    }

    /// Skip to the `ET` of a text object left open, applying its text state operators: they
    /// stay in force after it.
    fn skip_to_end(&mut self) -> Result<()> {
        self.open = false;

        for operator in &mut self.ops {
            match operator {
                Operator::EndText(_) => return Ok(()),
                Operator::Text(TextOperator::TextStateOperator(op)) => {
                    op.apply(self.graphics_state.text_state_mut())
                }
                _ => {}
            }
        }

        Err(ContentError::IncompleteTextObject)
    }
}

/// A [`TextObject`] and the operators that apply to it, up to its `ET`.
///
/// Iterates over the object's shown strings as [`Run`]s: each carries its start position in
/// default user space and its undecoded bytes. The font and other text state parameters in
/// force for the run just yielded are read from [`text_object`](Self::text_object).
///
/// Borrows the [`TextObjects`] it comes from: its operators, and the graphics state's text
/// state through the text object.
pub struct TextObjectStream<'a, Ops> {
    text_object: TextObject<'a>,
    ops: &'a mut Ops,
    /// Whether the owning [`TextObjects`] is inside a text object: lowered on `ET`.
    open: &'a mut bool,
}

impl<'a, Ops> TextObjectStream<'a, Ops> {
    /// The text object in its current state: the font, matrix and parameters that applied
    /// to the last element yielded by the stream.
    pub fn text_object(&self) -> &TextObject<'a> {
        &self.text_object
    }
}

impl<Ops> Iterator for TextObjectStream<'_, Ops>
where
    Ops: Iterator<Item = Operator>,
{
    type Item = Run;

    /// The next shown string, applying the text operators before it. Ends at `ET`, or with
    /// the operators.
    fn next(&mut self) -> Option<Run> {
        loop {
            if let Some(run) = self.text_object.next_run() {
                return Some(run);
            }

            let op = self.ops.next()?;
            match op {
                Operator::EndText(_) => {
                    *self.open = false;
                    return None;
                }
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

    /// The text objects of a content stream.
    fn objects(input: &[u8]) -> TextObjects<std::vec::IntoIter<Operator>> {
        TextObjects::new(operators(input).into_iter())
    }

    /// A shown string, decoded, with the font in force when it was yielded.
    type Shown = (String, Option<Font>);

    /// Every text object of a content stream, as the runs it yields, each with the font in
    /// force when it was yielded.
    fn runs(input: &[u8]) -> Result<Vec<Vec<Shown>>> {
        let mut objects = objects(input);
        let mut out = Vec::new();

        while let Some(mut stream) = objects.next_object()? {
            let mut runs = Vec::new();
            while let Some(Run { text, .. }) = stream.next() {
                runs.push((text.decode(), stream.text_object().parameters.font.clone()));
            }
            out.push(runs);
        }

        Ok(out)
    }

    fn font(name: &str, size: f32) -> Font {
        Font {
            name: Name(name.as_bytes().to_vec()),
            size,
        }
    }

    fn run(text: &str, name: &str, size: f32) -> Shown {
        (text.to_string(), Some(font(name, size)))
    }

    #[test]
    fn tf_before_bt_selects_the_font() {
        assert_eq!(
            runs(b"/F1 12 Tf BT (a) Tj ET").unwrap(),
            [vec![run("a", "F1", 12.0)]]
        );
    }

    #[test]
    fn tf_carries_over_to_the_next_text_object() {
        assert_eq!(
            runs(b"BT /F1 12 Tf (a) Tj ET BT (b) Tj ET").unwrap(),
            [vec![run("a", "F1", 12.0)], vec![run("b", "F1", 12.0)]]
        );
    }

    #[test]
    fn big_q_restores_the_text_state() {
        assert_eq!(
            runs(b"/F1 12 Tf q /F2 8 Tf BT (a) Tj ET Q BT (b) Tj ET").unwrap(),
            [vec![run("a", "F2", 8.0)], vec![run("b", "F1", 12.0)]]
        );
    }

    #[test]
    fn big_q_restores_the_ctm_and_the_text_state_together() {
        let mut objects = objects(b"q 2 0 0 2 0 0 cm /F1 12 Tf Q BT (a) Tj ET");
        assert_eq!(objects.graphics_state.ctm(), Matrix::coincident());
        assert!(matches!(
            objects.next_object(),
            Err(ContentError::NoFontSelected)
        ));
        assert_eq!(objects.graphics_state.ctm(), Matrix::coincident());
    }

    #[test]
    fn showing_text_with_no_font_anywhere_is_an_error() {
        assert!(matches!(
            objects(b"BT (a) Tj ET").next_object(),
            Err(ContentError::NoFontSelected)
        ));
    }

    #[test]
    fn showing_text_outside_bt_is_skipped() {
        assert_eq!(
            runs(b"(a) Tj BT /F1 12 Tf (b) Tj ET").unwrap(),
            [vec![run("b", "F1", 12.0)]]
        );
    }

    #[test]
    fn empty_text_object_is_skipped() {
        assert_eq!(
            runs(b"BT ET BT /F1 12 Tf (a) Tj ET").unwrap(),
            [vec![run("a", "F1", 12.0)]]
        );
    }

    #[test]
    fn empty_text_object_does_not_leak_its_position() {
        // Without the skip, the `Td` of the empty object would leak into the next one.
        let mut objects = objects(b"BT 0 10 Td ET BT /F1 12 Tf (a) Tj ET");
        let mut stream = objects.next_object().unwrap().unwrap();

        let Run { position, text } = stream.next().unwrap();
        assert_eq!(
            (position, text.decode()),
            (Point::new(0.0, 0.0), "a".to_string())
        );
        assert!(stream.next().is_none());
    }

    #[test]
    fn trailing_empty_text_object_ends_the_content() {
        assert!(objects(b"BT ET").next_object().unwrap().is_none());
    }

    #[test]
    fn missing_et_is_an_incomplete_text_object() {
        let mut objects = objects(b"BT /F1 12 Tf (a) Tj");

        let stream = objects.next_object().unwrap().unwrap();
        let texts: Vec<String> = stream.map(|Run { text, .. }| text.decode()).collect();
        assert_eq!(texts, ["a"]);

        assert!(matches!(
            objects.next_object(),
            Err(ContentError::IncompleteTextObject)
        ));
    }

    #[test]
    fn content_ending_right_after_bt_is_an_incomplete_text_object() {
        assert!(matches!(
            objects(b"BT /F1 12 Tf").next_object(),
            Err(ContentError::IncompleteTextObject)
        ));
    }

    #[test]
    fn dropped_stream_still_applies_the_rest_of_its_text_state() {
        let mut objects = objects(b"BT /F1 12 Tf (a) Tj /F2 8 Tf (b) Tj ET BT (c) Tj ET");

        drop(objects.next_object().unwrap().unwrap());
        let mut stream = objects.next_object().unwrap().unwrap();

        let Run { text, .. } = stream.next().unwrap();
        assert_eq!(text.decode(), "c");
        assert_eq!(stream.text_object().parameters.font, Some(font("F2", 8.0)));
    }

    #[test]
    fn second_tf_updates_the_font() {
        assert_eq!(
            runs(b"BT /F1 12 Tf (a) Tj /F2 10 Tf (b) Tj ET").unwrap(),
            [vec![run("a", "F1", 12.0), run("b", "F2", 10.0)]]
        );
    }

    #[test]
    fn runs_keep_the_bytes_that_decoding_conflates() {
        let input = b"BT /F1 12 Tf <FEFF0041> Tj (A) Tj ET";

        let mut objects = objects(input);
        let mut stream = objects.next_object().unwrap().unwrap();
        assert_eq!(
            stream.next(),
            Some(Run {
                position: Point::new(0.0, 0.0),
                text: PDFString(vec![0xFE, 0xFF, 0x00, 0x41]),
            })
        );
        assert_eq!(
            stream.next(),
            Some(Run {
                position: Point::new(0.0, 0.0),
                text: PDFString(b"A".to_vec()),
            })
        );
        assert!(stream.next().is_none());

        let texts: Vec<String> = runs(input)
            .unwrap()
            .concat()
            .into_iter()
            .map(|(text, _)| text)
            .collect();
        assert_eq!(texts, ["A", "A"]);
    }

    #[test]
    fn tj_offset_scales_with_font_size_and_horizontal_scaling() {
        // tx = -n/1000 · Tfs · Th: 500/1000 · 10 · 2. Glyph widths are unknown, so the
        // advance of `(A)` is not counted.
        let mut objects = objects(b"BT /F1 10 Tf 200 Tz [(A) -500 (B)] TJ ET");
        let stream = objects.next_object().unwrap().unwrap();
        let positions: Vec<Point<DefaultUser>> =
            stream.map(|Run { position, .. }| position).collect();
        assert_eq!(positions, [Point::new(0.0, 0.0), Point::new(10.0, 0.0)]);
    }

    #[test]
    fn big_td_sets_the_leading_used_by_t_star() {
        let mut objects = objects(b"BT /F1 12 Tf 0 -14 TD (a) Tj T* (b) Tj ET");
        let mut stream = objects.next_object().unwrap().unwrap();

        let Run { position, text } = stream.next().unwrap();
        assert_eq!(
            (position, text.decode()),
            (Point::new(0.0, -14.0), "a".to_string())
        );
        assert_eq!(stream.text_object().parameters.leading, 14.0);
        let Run { position, text } = stream.next().unwrap();
        assert_eq!(
            (position, text.decode()),
            (Point::new(0.0, -28.0), "b".to_string())
        );
    }

    #[test]
    fn double_quote_moves_to_the_next_line_and_sets_the_spacing() {
        let mut objects = objects(b"BT /F1 12 Tf 14 TL 3 1 (a) \" ET");
        let mut stream = objects.next_object().unwrap().unwrap();

        let Run { position, text } = stream.next().unwrap();
        assert_eq!(
            (position, text.decode()),
            (Point::new(0.0, -14.0), "a".to_string())
        );
        assert_eq!(stream.text_object().parameters.word_spacing, 3.0);
        assert_eq!(stream.text_object().parameters.character_spacing, 1.0);
    }
}

#[cfg(test)]
mod graphics_state_tests {
    use winnow::{ascii::multispace0, combinator::preceded, BStr};

    use super::*;

    fn objects(input: &[u8]) -> TextObjects<std::vec::IntoIter<Operator>> {
        let mut stream = BStr::new(input);
        let mut it =
            winnow::combinator::iterator(&mut stream, preceded(multispace0, Operator::extract));
        let ops: Vec<Operator> = (&mut it).collect();
        it.finish().unwrap();
        TextObjects::new(ops.into_iter())
    }

    /// Every run position of a content stream.
    fn positions(input: &[u8]) -> Vec<Point<DefaultUser>> {
        let mut objects = objects(input);
        let mut positions = Vec::new();

        while let Some(stream) = objects.next_object().unwrap() {
            positions.extend(stream.map(|Run { position, .. }| position));
        }

        positions
    }

    fn assert_close(Point { x, y, .. }: Point<DefaultUser>, (ex, ey): (f32, f32)) {
        assert!(
            (x - ex).abs() < 1e-3 && (y - ey).abs() < 1e-3,
            "expected ({ex}, {ey}), got ({x}, {y})"
        );
    }

    #[test]
    fn cm_before_bt_transforms_the_run_position() {
        // (5, 7) in text space, scaled by 2 and moved by (10, 20).
        let positions = positions(b"2 0 0 2 10 20 cm BT /F1 12 Tf 5 7 Td (a) Tj ET");
        assert_eq!(positions, vec![Point::new(20.0, 34.0)]);
    }

    #[test]
    fn q_and_big_q_restore_the_ctm_between_text_objects() {
        let positions =
            positions(b"q 1 0 0 1 100 0 cm BT /F1 12 Tf (a) Tj ET Q BT /F1 12 Tf (b) Tj ET");
        assert_eq!(
            positions,
            vec![Point::new(100.0, 0.0), Point::new(0.0, 0.0)]
        );
    }

    #[test]
    fn unbalanced_big_q_is_ignored() {
        let positions = positions(b"/F1 12 Tf Q 1 0 0 1 3 4 cm BT (a) Tj ET");
        assert_eq!(positions, vec![Point::new(3.0, 4.0)]);
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
        let mut objects = objects(SKIA_LINE);
        let mut stream = objects.next_object().unwrap().unwrap();
        stream.next().unwrap();

        let transformation = stream.text_object().transformation();

        // The `Tm` flip and the page flip cancel: text-space "up" is page "up", scaled by 0.75.
        assert_close(Point::new(transformation.a, transformation.d), (0.75, 0.75));
        assert_close(Point::new(transformation.b, transformation.c), (0.0, 0.0));
        assert_close(transformation.translation(), (132.4946, 695.1359));
    }
}
