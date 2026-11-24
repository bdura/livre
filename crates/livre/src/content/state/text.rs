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
        font::{Code, Font, Fonts},
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

/// A font size, `Tfs`: the factor applied to a glyph's width once the font has mapped it
/// to text space (ISO 32000-2, 9.4.4).
///
/// A newtype so that a size is never passed where a width or a spacing is expected.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct FontSize(pub f32);

/// The font selected by the `Tf` operator, borrowed from the page's [`Fonts`].
#[derive(Debug, Clone, PartialEq)]
pub struct SelectedFont<'f> {
    /// The resource name `Tf` selected the font by.
    pub name: Name,
    /// The font itself.
    pub font: &'f Font,
    /// The font size, a scaling factor applied to every glyph's size parameters.
    pub size: FontSize,
}

/// Text state parameters, as defined in section 9.3 of the PDF specification.
///
/// The selected font is borrowed from the page's [`Fonts`] for `'f`.
#[derive(Debug, Clone, PartialEq)]
pub struct TextStateParameters<'f> {
    /// Font and font size. `None` until a `Tf` operator selects one.
    pub font: Option<SelectedFont<'f>>,
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

impl Default for TextStateParameters<'_> {
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

impl<'f> TextStateParameters<'f> {
    /// Select the font and font size (`Tf`).
    pub fn set_font(&mut self, font: SelectedFont<'f>) {
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

/// One character code of a shown string: the glyph it selected, where that glyph starts and
/// ends in default user space, and the text it stands for.
///
/// Named after the specification's word for what a code stands for (ISO 32000-2, 9.2.2);
/// [`Glyph`](super::space::Glyph) is the glyph space marker.
#[derive(Debug, Clone, PartialEq)]
pub struct Character {
    /// The code the font read from the string.
    pub code: Code,
    /// The text position before the glyph: the point `(0, rise)` of text space, mapped
    /// through the text matrix and the CTM in force at `BT`.
    pub start: Point<DefaultUser>,
    /// The same point once the text matrix has advanced past the glyph, character and word
    /// spacing included.
    pub end: Point<DefaultUser>,
    /// What the code stands for, through the font's `/ToUnicode` or encoding. `None` when
    /// neither maps the code.
    pub text: Option<Box<str>>,
}

/// A shown string, as yielded by a [`TextObjectStream`].
#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    /// The start of the shown string in default user space: the point `(0, rise)` of text
    /// space, mapped through the text matrix and the CTM in force at `BT`.
    pub start: Point<DefaultUser>,
    /// The end of the shown string in default user space: the same point as
    /// [`start`](Self::start), after the text matrix has advanced past every glyph.
    pub end: Point<DefaultUser>,
    /// One per character code, in string order.
    pub characters: Vec<Character>,
    /// The characters' texts concatenated, U+FFFD in place of each code with no text.
    pub text: String,
}

/// The part of a [`TextObject`] that does not borrow the graphics state. The text object
/// is opened under a short borrow and rebuilt from this under the stream's, once it is
/// known to show text.
struct TextObjectState {
    matrix: Matrix<Text, User>,
    line_matrix: Matrix<Text, User>,
    /// The strings and `TJ` offsets of the last showing operator not yet yielded, in order.
    /// `Tj`, `'` and `"` buffer a single string.
    buffer: Option<VecDeque<TextArrayElement>>,
}

impl TextObjectState {
    /// The state at `BT`: identity text and line matrices, nothing to show yet.
    fn begin() -> Self {
        Self {
            matrix: Matrix::coincident(),
            line_matrix: Matrix::coincident(),
            buffer: None,
        }
    }
}

/// A text object: the state the text operators act on between `BT` and `ET`.
///
/// Its shown strings are read through the [`TextObjectStream`] that owns it.
pub struct TextObject<'a, 'f> {
    state: TextObjectState,
    /// The current transformation matrix in force at `BT`. Governs the transformation of
    /// user space to default user space. Constant for the whole text object, since `cm`,
    /// `q` and `Q` are not allowed inside one.
    pub ctm: Matrix<User, DefaultUser>,
    /// All text state parameters: font, leading, character spacing, word spacing, etc.
    ///
    /// Borrowed from the graphics state: the text state operators inside the object mutate
    /// it in place, so a `Tf` here is still in force in the next text object.
    pub parameters: &'a mut TextStateParameters<'f>,
    /// The page's fonts, which `Tf` selects from.
    pub fonts: &'f Fonts,
}

impl<'a, 'f> TextObject<'a, 'f> {
    /// The text object in `state`, under the CTM and text state in force at its `BT`.
    fn begin(
        state: TextObjectState,
        ctm: Matrix<User, DefaultUser>,
        parameters: &'a mut TextStateParameters<'f>,
        fonts: &'f Fonts,
    ) -> Self {
        Self {
            state,
            ctm,
            parameters,
            fonts,
        }
    }

    /// Text matrix, `Tm`. Governs the transformation of text space to user space. Showing
    /// operators and `TJ` offsets advance it; line moves reset it to the
    /// [line matrix](Self::line_matrix).
    pub fn matrix(&self) -> Matrix<Text, User> {
        self.state.matrix
    }

    /// Text line matrix, `Tlm`: the text matrix at the start of the current line. `Td`,
    /// `TD` and `T*` move from it rather than from the text matrix, so the advance along a
    /// line does not leak into the next one (ISO 32000-2, 9.4.2).
    pub fn line_matrix(&self) -> Matrix<Text, User> {
        self.state.line_matrix
    }

    /// The matrix mapping text space to default user space: `Tm × CTM`. Its translation is
    /// the current position at zero rise: [`Run::start`] maps `(0, rise)` through it.
    /// Its other components carry the orientation and scale of the text, before the font
    /// size is applied.
    pub fn transformation(&self) -> Matrix<Text, DefaultUser> {
        self.state.matrix.then(self.ctm)
    }

    /// Start a new line at `offset` from the start of the current one, expressed in text
    /// space (`Td`): translate the line matrix, and reset the text matrix to it.
    pub fn move_by(&mut self, offset: Displacement<Text>) {
        self.state.line_matrix.translate(offset);
        self.state.matrix = self.state.line_matrix;
    }
    /// Set both the text matrix and the line matrix (`Tm`).
    pub fn set_matrix(&mut self, matrix: Matrix<Text, User>) {
        self.state.matrix = matrix;
        self.state.line_matrix = matrix;
    }
    /// Start a new line, down by the leading from the start of the current one (`T*`).
    pub fn move_to_next_line(&mut self) {
        self.move_by(Displacement::new(0.0, -self.parameters.leading));
    }
    /// Buffer a shown string (`Tj`), to be yielded as the next run.
    pub fn add_text(&mut self, text: PDFString) {
        self.state.buffer = Some(VecDeque::from([TextArrayElement::Text(text)]));
    }
    /// Buffer the strings and offsets of a `TJ` array, to be yielded as runs in order, each
    /// offset moving the text matrix before the string that follows it.
    pub fn add_text_array(&mut self, array: Vec<TextArrayElement>) {
        self.state.buffer = Some(array.into());
    }

    /// The current text position in default user space: the point `(0, rise)` of text
    /// space through [`transformation`](Self::transformation). ISO 32000-2, 9.3.7: the rise
    /// shifts the baseline in text space.
    fn origin(&self) -> Point<DefaultUser> {
        let origin: Point<Text> = Point::new(0.0, self.parameters.rise);
        self.transformation().apply(origin)
    }

    /// Show `string`: advance the text matrix past each of its glyphs and return the run,
    /// from the text position before the first glyph to the one after the last, with a
    /// [`Character`] per code.
    ///
    /// The selected font splits the string into codes. Each code advances the text matrix
    /// by the displacement of ISO 32000-2, 9.4.4, along its font's
    /// [writing mode](Font::writing_mode), from the code's
    /// [glyph displacement](Font::glyph_displacement); `Tw` applies to the one-byte code 32
    /// only. With no font selected, nothing advances and the run has no
    /// characters.
    pub fn show(&mut self, string: &PDFString) -> Run {
        let start = self.origin();
        let mut characters = Vec::new();
        let mut text = String::new();

        if let Some(SelectedFont { font, size, .. }) = self.parameters.font {
            let mode = font.writing_mode();
            let TextStateParameters {
                character_spacing,
                word_spacing,
                horizontal_scaling,
                ..
            } = *self.parameters;

            for code in font.codes(string) {
                let start = self.origin();
                let spacing = if code.word_spacing_applies() {
                    character_spacing + word_spacing
                } else {
                    character_spacing
                };
                let shift = mode.advance(
                    font.glyph_displacement(code),
                    size,
                    spacing,
                    horizontal_scaling,
                );
                self.state.matrix.translate(shift);

                let decoded = font.decode(code);
                match &decoded {
                    Some(decoded) => text.push_str(decoded),
                    None => {
                        tracing::debug!("No text for code {code:?}: U+FFFD in its place");
                        text.push(char::REPLACEMENT_CHARACTER);
                    }
                }
                characters.push(Character {
                    code,
                    start,
                    end: self.origin(),
                    text: decoded.map(Into::into),
                });
            }
        }

        Run {
            start,
            end: self.origin(),
            characters,
            text,
        }
    }

    /// The next run buffered by the last showing operator, if any. Consumes the `TJ`
    /// offsets met before it, moving the text matrix.
    fn next_run(&mut self) -> Option<Run> {
        let buffer = self.state.buffer.as_mut()?;
        loop {
            match buffer.pop_front()? {
                TextArrayElement::Text(text) => return Some(self.show(&text)),
                TextArrayElement::Offset(offset) => {
                    // With no font selected the size is zero: the offset moves nothing.
                    if let Some(SelectedFont { font, size, .. }) = &self.parameters.font {
                        let shift = font.writing_mode().adjustment(
                            offset,
                            *size,
                            self.parameters.horizontal_scaling,
                        );
                        self.state.matrix.translate(shift);
                    }
                }
            }
        }
    }
}

/// The text objects of a content stream, in order.
///
/// Owns the operator iterator and the graphics state it builds up: `q`, `Q`, `cm` and the
/// text state operators (`Tf`, `Tc`, ...) met outside `BT` update it, and each text object
/// starts from the CTM and text state then in force. Text positioning and showing
/// operators outside `BT` are illegal and skipped with a warning.
///
/// Borrows the page's [`Fonts`] for `'f`: `Tf` selects from them, and the text state keeps
/// a borrow of the selected font.
///
/// Not an [`Iterator`], since each item borrows it:
///
/// ```
/// # use livre::{content::{operators::Operator, Fonts, Run, TextObjects}, extraction::{extract, multicomment0, Extract, Name}};
/// # use winnow::{combinator::{iterator, preceded}, BStr};
/// # let font = extract(&mut BStr::new(b"<</Type/Font/Subtype/Type1/BaseFont/Helvetica/FirstChar 32/LastChar 32/Widths[278]>>")).unwrap();
/// # let fonts: Fonts = [(Name::from("F1"), font)].into_iter().collect();
/// # let mut content = BStr::new(b"/F1 12 Tf BT 72 700 Td (Hello) Tj ET");
/// # let mut operators = iterator(&mut content, preceded(multicomment0, Operator::extract));
/// // `fonts` comes from `Page::fonts`.
/// let mut objects = TextObjects::new(&mut operators, &fonts);
/// while let Some(stream) = objects.next_object()? {
///     for Run { start, text, .. } in stream {
/// #       assert_eq!((start.x, start.y, text.as_str()), (72.0, 700.0, "Hello"));
///         /* ... */
///     }
/// }
/// # Ok::<(), livre::content::ContentError>(())
/// ```
pub struct TextObjects<'f, Ops> {
    ops: Ops,
    fonts: &'f Fonts,
    graphics_state: GraphicsState<'f>,
    /// Inside a text object whose `ET` has not been read yet: its stream was dropped before
    /// the end, or opening it failed.
    open: bool,
    /// The error that ended the last stream early, returned by the next
    /// [`next_object`](Self::next_object).
    error: Option<ContentError>,
}

impl<'f, Ops> TextObjects<'f, Ops>
where
    Ops: Iterator<Item = Operator>,
{
    /// Start at the top of a content stream, with the default graphics state. `Tf` selects
    /// from `fonts`, the fonts of the page the content stream belongs to.
    pub fn new(ops: Ops, fonts: &'f Fonts) -> Self {
        Self {
            ops,
            fonts,
            graphics_state: GraphicsState::default(),
            open: false,
            error: None,
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
    /// - [`ContentError::UnknownFont`] if a `Tf` names a font absent from the fonts. Met
    ///   outside a stream, it is returned at once. Met by a stream, it ends the stream, whose
    ///   items are infallible [`Run`]s: the error is kept and returned by the next call,
    ///   the way winnow's `iterator` reports its error through `finish` after the loop. The
    ///   call after that resumes past the text object's `ET`.
    pub fn next_object(&mut self) -> Result<Option<TextObjectStream<'_, 'f, Ops>>> {
        if let Some(error) = self.error.take() {
            return Err(error);
        }

        if self.open {
            self.skip_to_end()?;
        }

        loop {
            match self.ops.next() {
                None => return Ok(None),
                Some(Operator::BeginText(_)) => {
                    self.open = true;
                    if let Some(state) = self.start()? {
                        let text_object = TextObject::begin(
                            state,
                            self.graphics_state.ctm(),
                            self.graphics_state.text_state_mut(),
                            self.fonts,
                        );
                        return Ok(Some(TextObjectStream {
                            text_object,
                            ops: &mut self.ops,
                            open: &mut self.open,
                            error: &mut self.error,
                        }));
                    }
                }
                Some(Operator::GraphicsState(op)) => op.apply(&mut self.graphics_state),
                Some(Operator::Text(TextOperator::TextStateOperator(op))) => {
                    op.apply(self.graphics_state.text_state_mut(), self.fonts)?
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
    /// the text object's state as it shows it. `None` if `ET` comes first: the object is
    /// empty.
    fn start(&mut self) -> Result<Option<TextObjectState>> {
        let mut text_object = TextObject::begin(
            TextObjectState::begin(),
            self.graphics_state.ctm(),
            self.graphics_state.text_state_mut(),
            self.fonts,
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
                    return Ok(Some(text_object.state));
                }
                Operator::Text(op) => op.apply(&mut text_object)?,
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
                    op.apply(self.graphics_state.text_state_mut(), self.fonts)?
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
/// default user space, its characters and its decoded text. The font and other text state
/// parameters in force for the run just yielded are read from
/// [`text_object`](Self::text_object).
///
/// Borrows the [`TextObjects`] it comes from: its operators, and the graphics state's text
/// state through the text object. A `Tf` naming an unknown font ends the stream; the
/// error is returned by the next [`TextObjects::next_object`].
pub struct TextObjectStream<'a, 'f, Ops> {
    text_object: TextObject<'a, 'f>,
    ops: &'a mut Ops,
    /// Whether the owning [`TextObjects`] is inside a text object: lowered on `ET`.
    open: &'a mut bool,
    /// Where the error that ends the stream early is kept for the owning [`TextObjects`].
    error: &'a mut Option<ContentError>,
}

impl<'a, 'f, Ops> TextObjectStream<'a, 'f, Ops> {
    /// The text object in its current state: the font and parameters that applied to the
    /// last run yielded, and the text matrix advanced past it, at that run's [`Run::end`].
    pub fn text_object(&self) -> &TextObject<'a, 'f> {
        &self.text_object
    }
}

impl<Ops> Iterator for TextObjectStream<'_, '_, Ops>
where
    Ops: Iterator<Item = Operator>,
{
    type Item = Run;

    /// The next shown string, applying the text operators before it. Ends at `ET`, with
    /// the operators, or at an operator that fails to apply.
    fn next(&mut self) -> Option<Run> {
        if self.error.is_some() {
            return None;
        }

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
                Operator::Text(op) => {
                    if let Err(error) = op.apply(&mut self.text_object) {
                        *self.error = Some(error);
                        return None;
                    }
                }
                _ => {
                    tracing::debug!("Skipping operator: {:?}", op);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use winnow::{combinator::preceded, BStr};

    use crate::extraction::multicomment0;

    use std::sync::LazyLock;

    use super::*;
    use crate::content::font::tests::{fonts, HELVETICA, HELVETICA_STANDARD};

    /// Parse a content stream into its operators.
    pub(super) fn operators(input: &[u8]) -> Vec<Operator> {
        let mut stream = BStr::new(input);
        let mut it =
            winnow::combinator::iterator(&mut stream, preceded(multicomment0, Operator::extract));
        let ops: Vec<Operator> = (&mut it).collect();
        it.finish().unwrap();
        ops
    }

    /// The fonts the tests' content streams select: `F1`, `F2` and `F4`.
    pub(super) static PAGE_FONTS: LazyLock<Fonts> =
        LazyLock::new(|| fonts(&[("F1", HELVETICA), ("F2", HELVETICA), ("F4", HELVETICA)]));

    /// The text objects of a content stream, selecting from `fonts`.
    fn objects_with<'f>(
        input: &[u8],
        fonts: &'f Fonts,
    ) -> TextObjects<'f, std::vec::IntoIter<Operator>> {
        TextObjects::new(operators(input).into_iter(), fonts)
    }

    /// The text objects of a content stream, selecting from [`PAGE_FONTS`].
    fn objects(input: &[u8]) -> TextObjects<'static, std::vec::IntoIter<Operator>> {
        objects_with(input, &PAGE_FONTS)
    }

    /// The resource name and size of a selected font.
    type Selected = (Name, FontSize);

    /// A shown string, decoded, with the font in force when it was yielded.
    type Shown = (String, Option<Selected>);

    fn selected(parameters: &TextStateParameters) -> Option<Selected> {
        parameters.font.as_ref().map(|f| (f.name.clone(), f.size))
    }

    /// Every text object of a content stream, as the runs it yields, each with the font in
    /// force when it was yielded.
    fn runs(input: &[u8]) -> Result<Vec<Vec<Shown>>> {
        let mut objects = objects(input);
        let mut out = Vec::new();

        while let Some(mut stream) = objects.next_object()? {
            let mut runs = Vec::new();
            while let Some(Run { text, .. }) = stream.next() {
                runs.push((text, selected(stream.text_object().parameters)));
            }
            out.push(runs);
        }

        Ok(out)
    }

    fn font(name: &str, size: f32) -> Selected {
        (Name::from(name), FontSize(size))
    }

    fn run(text: &str, name: &str, size: f32) -> Shown {
        (text.to_string(), Some(font(name, size)))
    }

    #[test]
    fn tf_selects_the_font_from_the_page_fonts() {
        let fonts = fonts(&[("F1", HELVETICA)]);
        let mut objects = objects_with(b"/F1 12 Tf BT (a) Tj ET", &fonts);
        let mut stream = objects.next_object().unwrap().unwrap();

        stream.next().unwrap();
        let selected = stream.text_object().parameters.font.as_ref().unwrap();
        assert_eq!(selected.name, Name::from("F1"));
        assert!(std::ptr::eq(
            selected.font,
            fonts.get(&"F1".into()).unwrap()
        ));
        assert_eq!(selected.size, FontSize(12.0));
    }

    #[test]
    fn tf_with_an_unknown_font_outside_bt_is_an_error() {
        let fonts = fonts(&[("F1", HELVETICA)]);
        let mut objects = objects_with(b"/F9 12 Tf", &fonts);

        assert!(matches!(
            objects.next_object(),
            Err(ContentError::UnknownFont(name)) if name == Name::from("F9")
        ));
    }

    #[test]
    fn tf_with_an_unknown_font_inside_bt_ends_the_stream_then_errors() {
        let fonts = fonts(&[("F1", HELVETICA)]);
        let mut objects = objects_with(b"BT /F1 12 Tf (a) Tj /F9 10 Tf (b) Tj ET", &fonts);

        let stream = objects.next_object().unwrap().unwrap();
        let texts: Vec<String> = stream.map(|Run { text, .. }| text).collect();
        assert_eq!(texts, ["a"]);

        assert!(matches!(
            objects.next_object(),
            Err(ContentError::UnknownFont(name)) if name == Name::from("F9")
        ));
    }

    #[test]
    fn tf_with_an_unknown_font_before_the_first_run_is_an_error() {
        let fonts = fonts(&[("F1", HELVETICA)]);
        let mut objects = objects_with(b"BT /F9 10 Tf (b) Tj ET", &fonts);

        assert!(matches!(
            objects.next_object(),
            Err(ContentError::UnknownFont(name)) if name == Name::from("F9")
        ));
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

        let Run { start, text, .. } = stream.next().unwrap();
        assert_eq!((start, text), (Point::new(0.0, 0.0), "a".to_string()));
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
        let texts: Vec<String> = stream.map(|Run { text, .. }| text).collect();
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
        assert_eq!(text, "c");
        assert_eq!(
            selected(stream.text_object().parameters),
            Some(font("F2", 8.0))
        );
    }

    #[test]
    fn second_tf_updates_the_font() {
        assert_eq!(
            runs(b"BT /F1 12 Tf (a) Tj /F2 10 Tf (b) Tj ET").unwrap(),
            [vec![run("a", "F1", 12.0), run("b", "F2", 10.0)]]
        );
    }

    #[test]
    fn tj_offset_scales_with_font_size_and_horizontal_scaling() {
        // tx = -n/1000 · Tfs · Th: 500/1000 · 10 · 2. `A` is zero wide in `F1`, so
        // only the offset moves `(B)`.
        let mut objects = objects(b"BT /F1 10 Tf 200 Tz [(A) -500 (B)] TJ ET");
        let stream = objects.next_object().unwrap().unwrap();
        let positions: Vec<Point<DefaultUser>> = stream.map(|Run { start, .. }| start).collect();
        assert_eq!(positions, [Point::new(0.0, 0.0), Point::new(10.0, 0.0)]);
    }

    #[test]
    fn big_td_sets_the_leading_used_by_t_star() {
        let mut objects = objects(b"BT /F1 12 Tf 0 -14 TD (a) Tj T* (b) Tj ET");
        let mut stream = objects.next_object().unwrap().unwrap();

        let Run { start, text, .. } = stream.next().unwrap();
        assert_eq!((start, text), (Point::new(0.0, -14.0), "a".to_string()));
        assert_eq!(stream.text_object().parameters.leading, 14.0);
        let Run { start, text, .. } = stream.next().unwrap();
        assert_eq!((start, text), (Point::new(0.0, -28.0), "b".to_string()));
    }

    #[test]
    fn double_quote_moves_to_the_next_line_and_sets_the_spacing() {
        let mut objects = objects(b"BT /F1 12 Tf 14 TL 3 1 (a) \" ET");
        let mut stream = objects.next_object().unwrap().unwrap();

        let Run { start, text, .. } = stream.next().unwrap();
        assert_eq!((start, text), (Point::new(0.0, -14.0), "a".to_string()));
        assert_eq!(stream.text_object().parameters.word_spacing, 3.0);
        assert_eq!(stream.text_object().parameters.character_spacing, 1.0);
    }

    #[test]
    fn text_object_exposes_its_matrices() {
        // The standard-14 widths, so that `a` and `b` advance the text matrix.
        let fonts = fonts(&[("F1", HELVETICA_STANDARD)]);
        let mut objects = objects_with(b"/F1 10 Tf BT 10 20 Td (ab) Tj ET", &fonts);
        let mut stream = objects.next_object().unwrap().unwrap();
        let Run { end, .. } = stream.next().unwrap();

        let text_object = stream.text_object();
        assert_eq!(
            text_object.line_matrix().translation(),
            Point::new(10.0, 20.0)
        );
        // Identity CTM and zero rise: the text matrix's translation is the run's end.
        let Point { x, y, .. } = text_object.matrix().translation();
        assert!(
            (x - end.x).abs() < 1e-3 && (y - end.y).abs() < 1e-3,
            "expected ({}, {}), got ({x}, {y})",
            end.x,
            end.y
        );
        assert!(x > 10.0, "the text matrix did not advance past the run");
    }
}

#[cfg(test)]
mod graphics_state_tests {
    use winnow::{combinator::preceded, BStr};

    use crate::extraction::multicomment0;

    use super::*;

    /// The text objects of a content stream, selecting `F1` and `F4` from the page fonts.
    fn objects(input: &[u8]) -> TextObjects<'static, std::vec::IntoIter<Operator>> {
        let mut stream = BStr::new(input);
        let mut it =
            winnow::combinator::iterator(&mut stream, preceded(multicomment0, Operator::extract));
        let ops: Vec<Operator> = (&mut it).collect();
        it.finish().unwrap();
        TextObjects::new(ops.into_iter(), &super::tests::PAGE_FONTS)
    }

    /// Every run start of a content stream.
    fn positions(input: &[u8]) -> Vec<Point<DefaultUser>> {
        let mut objects = objects(input);
        let mut positions = Vec::new();

        while let Some(stream) = objects.next_object().unwrap() {
            positions.extend(stream.map(|Run { start, .. }| start));
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
    fn td_after_tj_starts_from_the_line_matrix() {
        // `Td` moves from the line matrix, which the `TJ` offset of -500 · 10 / 1000 = 5
        // does not shift: `(C)` is at (0, -12), not (5, -12).
        let positions = positions(b"BT /F1 10 Tf [(A) -500 (B)] TJ 0 -12 Td (C) Tj ET");
        assert_eq!(positions[2], Point::new(0.0, -12.0));
    }

    #[test]
    fn td_after_tm_starts_from_the_set_matrix() {
        let positions = positions(b"BT /F1 10 Tf 1 0 0 1 10 10 Tm 5 0 Td (a) Tj ET");
        assert_eq!(positions, vec![Point::new(15.0, 10.0)]);
    }

    #[test]
    fn td_after_tm_ignores_the_tj_offsets_between_them() {
        let positions =
            positions(b"BT /F1 10 Tf 1 0 0 1 10 10 Tm [(a) -500 (b)] TJ 5 0 Td (c) Tj ET");
        assert_eq!(positions[2], Point::new(15.0, 10.0));
    }

    #[test]
    fn t_star_starts_from_the_line_matrix() {
        // `TD` sets the leading to 14; the `TJ` offset moves the text matrix by 5 but not
        // the line matrix, which `T*` moves from.
        let positions =
            positions(b"BT /F1 10 Tf 1 0 0 1 10 100 Tm 0 -14 TD [(a) -500 (b)] TJ T* (c) Tj ET");
        assert_eq!(
            positions,
            vec![
                Point::new(10.0, 86.0),
                Point::new(15.0, 86.0),
                Point::new(10.0, 72.0)
            ]
        );
    }

    #[test]
    fn rise_offsets_the_reported_position() {
        let positions = positions(b"BT /F1 10 Tf 3 Ts (a) Tj ET");
        assert_eq!(positions, vec![Point::new(0.0, 3.0)]);
    }

    #[test]
    fn rise_is_in_text_space() {
        let positions = positions(b"BT /F1 10 Tf 2 0 0 2 0 0 Tm 3 Ts (a) Tj ET");
        assert_eq!(positions, vec![Point::new(0.0, 6.0)]);
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

#[cfg(test)]
mod advance_tests {
    use winnow::{combinator::preceded, BStr};

    use crate::extraction::multicomment0;

    use super::*;
    use crate::content::font::tests::fonts;

    /// Helvetica from code 32 to 66: space is 278, `A` and `B` are 667, the codes between
    /// them are zero wide.
    fn helvetica() -> Vec<u8> {
        let mut widths = vec!["278"];
        widths.extend(std::iter::repeat_n("0", 32));
        widths.extend(["667", "667"]);
        format!(
            "<</Type/Font/Subtype/Type1/BaseFont/Helvetica/FirstChar 32/LastChar 66/Widths[{}]>>",
            widths.join(" ")
        )
        .into_bytes()
    }

    /// `F1` is [`helvetica`]; `F2` an `Identity-H` `Type0` font whose `A`
    /// (CID 65) is 500 wide and every other CID zero; `F3` a `Type3` font whose `a` is
    /// 50 units of a 1/100 glyph space; `F4` a font whose only width is `a`'s, with a
    /// `/MissingWidth` of 250.
    static FONTS: std::sync::LazyLock<Fonts> = std::sync::LazyLock::new(|| {
        fonts(&[
            ("F1", &helvetica()),
            (
                "F2",
                b"<</Type/Font/Subtype/Type0/BaseFont/Calibri/Encoding/Identity-H\
                  /DescendantFonts[<</Type/Font/Subtype/CIDFontType2/BaseFont/Calibri/DW 0/W[65[500]]>>]>>",
            ),
            (
                "F3",
                b"<</Type/Font/Subtype/Type3/FontMatrix[0.01 0 0 0.01 0 0]/FirstChar 97/LastChar 97/Widths[50]>>",
            ),
            (
                "F4",
                b"<</Type/Font/Subtype/Type1/BaseFont/Helvetica/FirstChar 97/LastChar 97/Widths[500]/FontDescriptor<</MissingWidth 250>>>>",
            ),
        ])
    });

    /// Every run of a content stream, selecting from [`FONTS`].
    fn runs(input: &[u8]) -> Vec<Run> {
        let mut stream = BStr::new(input);
        let mut it =
            winnow::combinator::iterator(&mut stream, preceded(multicomment0, Operator::extract));
        let ops: Vec<Operator> = (&mut it).collect();
        it.finish().unwrap();

        let mut objects = TextObjects::new(ops.into_iter(), &FONTS);
        let mut runs = Vec::new();
        while let Some(stream) = objects.next_object().unwrap() {
            runs.extend(stream);
        }
        runs
    }

    /// The single run of a content stream.
    fn run(input: &[u8]) -> Run {
        let runs = runs(input);
        assert_eq!(runs.len(), 1, "{runs:?}");
        runs.into_iter().next().unwrap()
    }

    fn assert_close(Point { x, y, .. }: Point<DefaultUser>, (ex, ey): (f32, f32)) {
        assert!(
            (x - ex).abs() < 1e-4 && (y - ey).abs() < 1e-4,
            "expected ({ex}, {ey}), got ({x}, {y})"
        );
    }

    #[test]
    fn width_advances_by_the_font_size() {
        // 667/1000 · 10.
        let run = run(b"BT /F1 10 Tf (A) Tj ET");
        assert_close(run.start, (0.0, 0.0));
        assert_close(run.end, (6.67, 0.0));
    }

    #[test]
    fn tj_offset_comes_after_the_advance() {
        // `A`: 667/1000 · 10 = 6.67; offset: 500/1000 · 10 = 5; `B` from 11.67 to
        // 11.67 + 6.67 = 18.34.
        let runs = runs(b"BT /F1 10 Tf [(A) -500 (B)] TJ ET");
        assert_close(runs[0].end, (6.67, 0.0));
        assert_close(runs[1].start, (11.67, 0.0));
        assert_close(runs[1].end, (18.34, 0.0));
    }

    #[test]
    fn character_spacing_applies_to_every_glyph() {
        // (667/1000 · 10 + 1) · 2 = 13.34 + 2.
        let run = run(b"BT /F1 10 Tf 1 Tc (AB) Tj ET");
        assert_close(run.end, (13.34 + 2.0, 0.0));
    }

    #[test]
    fn word_spacing_applies_to_the_one_byte_space() {
        // 667/1000 · 10 + (278/1000 · 10 + 2) + 667/1000 · 10.
        let run = run(b"BT /F1 10 Tf 2 Tw (A B) Tj ET");
        assert_close(run.end, (6.67 + 2.78 + 2.0 + 6.67, 0.0));
    }

    #[test]
    fn word_spacing_skips_two_byte_codes() {
        // `A ` is the two-byte code 0x4120 and the odd `B` is dropped; `<0020>` is the
        // two-byte code 32. Both take `/DW` 0, and `Tw` applies to neither: 0.
        let runs = runs(b"BT /F2 10 Tf 2 Tw (A B) Tj <0020> Tj ET");
        assert_close(runs[0].end, (0.0, 0.0));
        assert_close(runs[1].end, (0.0, 0.0));
    }

    #[test]
    fn horizontal_scaling_scales_the_advance() {
        // 667/1000 · 10 · 50/100.
        let run = run(b"BT /F1 10 Tf 50 Tz (A) Tj ET");
        assert_close(run.end, (3.335, 0.0));
    }

    #[test]
    fn type3_widths_go_through_the_font_matrix() {
        // 50 · 0.01 · 10.
        let run = run(b"BT /F3 10 Tf (a) Tj ET");
        assert_close(run.end, (5.0, 0.0));
    }

    #[test]
    fn codes_outside_the_table_get_the_missing_width() {
        // `b` is past `/LastChar`: 250/1000 · 10.
        let run = run(b"BT /F4 10 Tf (b) Tj ET");
        assert_close(run.end, (2.5, 0.0));
    }

    #[test]
    #[tracing_test::traced_test]
    fn composite_fonts_advance_by_their_cid_widths() {
        // `A` is 500/1000 · 10, `B` past `/W` gets `/DW` 0; plus 1 Tc each.
        let run = run(b"BT /F2 10 Tf 1 Tc 3 0 Td (\\000A\\000B) Tj ET");
        assert_close(run.start, (3.0, 0.0));
        assert_close(run.end, (3.0 + 5.0 + 1.0 + 1.0, 0.0));
        // `FONTS` may be built in this test, so the `Type3` font's own warning can show.
        logs_assert(|lines: &[&str]| {
            match lines.iter().find(|line| {
                line.contains(" WARN ") && !line.contains("livre::content::font::type3")
            }) {
                Some(line) => Err(format!("unexpected warning: {line}")),
                None => Ok(()),
            }
        });
    }

    #[test]
    fn rise_offsets_start_and_end_but_not_the_advance() {
        // Both at y = 3; x as without the rise, 667/1000 · 10.
        let run = run(b"BT /F1 10 Tf 3 Ts (A) Tj ET");
        assert_close(run.start, (0.0, 3.0));
        assert_close(run.end, (6.67, 3.0));
    }

    #[test]
    fn advance_carries_to_the_next_string() {
        // `'` moves to the next line; `Tj` after a `Tj` starts at the end of the first.
        let runs = runs(b"BT /F1 10 Tf 12 TL (A) Tj (B) Tj (A) ' ET");
        assert_close(runs[1].start, (6.67, 0.0));
        assert_close(runs[1].end, (13.34, 0.0));
        assert_close(runs[2].start, (0.0, -12.0));
        assert_close(runs[2].end, (6.67, -12.0));
    }
}

#[cfg(test)]
mod character_tests {
    use super::*;
    use crate::content::font::tests::fonts;

    /// `F1` is Helvetica from code 65 to 66 with `/Widths`: `A` and `B` are 667. Its codes
    /// decode through `StandardEncoding`.
    static FONTS: std::sync::LazyLock<Fonts> = std::sync::LazyLock::new(|| {
        fonts(&[(
            "F1",
            b"<</Type/Font/Subtype/Type1/BaseFont/Helvetica/FirstChar 65/LastChar 66/Widths[667 667]>>",
        )])
    });

    /// Every run of a content stream, selecting from [`FONTS`].
    fn runs(input: &[u8]) -> Vec<Run> {
        let mut objects = TextObjects::new(super::tests::operators(input).into_iter(), &FONTS);
        let mut runs = Vec::new();
        while let Some(stream) = objects.next_object().unwrap() {
            runs.extend(stream);
        }
        runs
    }

    /// The single run of a content stream.
    fn run(input: &[u8]) -> Run {
        let runs = runs(input);
        assert_eq!(runs.len(), 1, "{runs:?}");
        runs.into_iter().next().unwrap()
    }

    fn assert_close(Point { x, y, .. }: Point<DefaultUser>, (ex, ey): (f32, f32)) {
        assert!(
            (x - ex).abs() < 1e-4 && (y - ey).abs() < 1e-4,
            "expected ({ex}, {ey}), got ({x}, {y})"
        );
    }

    #[test]
    fn each_code_is_a_character_from_its_start_to_its_end() {
        // 667/1000 · 12 = 8.004 per glyph.
        let run = run(b"/F1 12 Tf BT 72 700 Td (AB) Tj ET");

        let [a, b] = &run.characters[..] else {
            panic!("expected two characters, got {:?}", run.characters);
        };
        assert_eq!((a.code, b.code), (Code::one_byte(65), Code::one_byte(66)));
        assert_close(a.start, (72.0, 700.0));
        assert_close(a.end, (80.004, 700.0));
        assert_eq!(a.end, b.start);
        assert_close(b.end, (88.008, 700.0));
        assert_eq!(
            (a.text.as_deref(), b.text.as_deref()),
            (Some("A"), Some("B"))
        );
        assert_eq!(run.text, "AB");
        assert_eq!(run.start, a.start);
        assert_eq!(run.end, b.end);
    }

    #[test]
    fn character_end_includes_the_character_spacing() {
        // 8.004 + 1 per glyph.
        let run = run(b"/F1 12 Tf BT 72 700 Td 1 Tc (AB) Tj ET");
        assert_close(run.characters[0].end, (81.004, 700.0));
        assert_close(run.characters[1].start, (81.004, 700.0));
        assert_close(run.characters[1].end, (90.008, 700.0));
    }

    #[test]
    fn a_code_with_no_text_is_a_replacement_character() {
        // `StandardEncoding` leaves 0xFE, 0xFF and 0x00 undefined.
        let run = run(b"/F1 12 Tf BT <FEFF0041> Tj ET");
        assert_eq!(run.characters.len(), 4);
        assert_eq!(run.characters[0].text, None);
        assert_eq!(run.characters[3].text.as_deref(), Some("A"));
        assert_eq!(run.text, "\u{FFFD}\u{FFFD}\u{FFFD}A");
    }

    #[test]
    fn tj_offset_moves_the_next_run_first_character() {
        // `A` ends at 8.004; the offset adds 500/1000 · 12 = 6.
        let runs = runs(b"/F1 12 Tf BT [(A) -500 (B)] TJ ET");
        assert_close(runs[1].characters[0].start, (14.004, 0.0));
        assert_close(runs[1].characters[0].end, (22.008, 0.0));
    }

    #[test]
    fn character_starts_carry_the_rise() {
        let run = run(b"/F1 12 Tf BT 5 Ts (AB) Tj ET");
        assert_close(run.characters[0].start, (0.0, 5.0));
        assert_close(run.characters[1].start, (8.004, 5.0));
        assert_close(run.characters[1].end, (16.008, 5.0));
    }

    #[test]
    fn character_positions_are_in_default_user_space() {
        let run = run(b"q 2 0 0 2 0 0 cm /F1 12 Tf BT 10 20 Td (AB) Tj ET Q");
        assert_close(run.characters[0].start, (20.0, 40.0));
        assert_close(run.characters[0].end, (36.008, 40.0));
        assert_close(run.characters[1].end, (52.016, 40.0));
    }
}

#[cfg(test)]
mod vertical_tests {
    use super::*;
    use crate::content::font::tests::composite_f3;

    /// `F3` is `/F3` of `composite.pdf`: `Identity-V`, `X` (CID 1) advancing by `-500` and
    /// `Y` (CID 2) by the default `/DW2` `-1000`.
    static FONTS: std::sync::LazyLock<Fonts> =
        std::sync::LazyLock::new(|| [(Name::from("F3"), composite_f3())].into_iter().collect());

    /// Every run of a content stream, selecting from [`FONTS`].
    fn runs(input: &[u8]) -> Vec<Run> {
        let mut objects = TextObjects::new(super::tests::operators(input).into_iter(), &FONTS);
        let mut runs = Vec::new();
        while let Some(stream) = objects.next_object().unwrap() {
            runs.extend(stream);
        }
        runs
    }

    /// The single run of a content stream.
    fn run(input: &[u8]) -> Run {
        let runs = runs(input);
        assert_eq!(runs.len(), 1, "{runs:?}");
        runs.into_iter().next().unwrap()
    }

    fn assert_close(Point { x, y, .. }: Point<DefaultUser>, (ex, ey): (f32, f32)) {
        assert!(
            (x - ex).abs() < 1e-4 && (y - ey).abs() < 1e-4,
            "expected ({ex}, {ey}), got ({x}, {y})"
        );
    }

    #[test]
    fn vertical_glyphs_advance_down_by_their_w1() {
        // `X`: -500/1000 · 12 = -6; `Y`: -1000/1000 · 12 = -12.
        let run = run(b"BT /F3 12 Tf 300 700 Td <00010002> Tj ET");
        assert_eq!(run.text, "XY");
        let [x, y] = run.characters.as_slice() else {
            panic!("expected two characters, got {:?}", run.characters);
        };
        assert_eq!(x.text.as_deref(), Some("X"));
        assert_close(x.start, (300.0, 700.0));
        assert_close(x.end, (300.0, 694.0));
        assert_eq!(y.text.as_deref(), Some("Y"));
        assert_close(y.start, (300.0, 694.0));
        assert_close(y.end, (300.0, 682.0));
        assert_close(run.start, (300.0, 700.0));
        assert_close(run.end, (300.0, 682.0));
    }

    #[test]
    fn character_spacing_moves_vertical_glyphs_up() {
        // Each `Y`: -12 + 1 = -11.
        let run = run(b"BT /F3 12 Tf 1 Tc 300 600 Td <00020002> Tj ET");
        assert_close(run.start, (300.0, 600.0));
        assert_close(run.end, (300.0, 578.0));
    }

    #[test]
    fn horizontal_scaling_leaves_vertical_runs_alone() {
        let runs = runs(
            b"BT /F3 12 Tf 50 Tz 300 700 Td <00010002> Tj ET \
              BT /F3 12 Tf 1 Tc 300 600 Td <00020002> Tj ET",
        );
        assert_close(runs[0].end, (300.0, 682.0));
        assert_close(runs[1].end, (300.0, 578.0));
    }

    #[test]
    fn tj_offset_moves_along_y() {
        // `X` ends at 694; a positive offset moves down (ISO 32000-2, Table 107):
        // 500/1000 · 12 = 6, unscaled by `Tz`.
        let down = runs(b"BT /F3 12 Tf 50 Tz 300 700 Td [<0001> 500 <0002>] TJ ET");
        assert_close(down[0].end, (300.0, 694.0));
        assert_close(down[1].start, (300.0, 688.0));
        // And a negative one up: back to 700.
        let up = runs(b"BT /F3 12 Tf 300 700 Td [<0001> -500 <0002>] TJ ET");
        assert_close(up[1].start, (300.0, 700.0));
    }
}
