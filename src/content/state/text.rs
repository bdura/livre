//! Text state.
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

use crate::{
    content::operators::{
        PreTextOperation, SetFontAndFontSize, TextOperation, TextOperator, TextStateOperator,
    },
    debug,
    extraction::{Name, PDFString},
};

use super::super::{operators::RenderingMode, Operator, TextArrayElement};

pub struct TextStateParameters {
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
    /// > Word spacing shall be applied to every occurrence of the single-byte character code 32
    /// > in a string when using a simple font (including Type 3) or a composite font that defines
    /// > code 32 as a single-byte code. It shall not apply to occurrences of the byte value 32
    /// > in multiple-byte codes
    pub word_spacing: f32,
    /// Horizontal scaling adjusts the width of glyphs by stretching or compressing them in the
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
            character_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            leading: 0.0,
            rendering_mode: RenderingMode::Fill,
            rise: 0.0,
        }
    }
}

pub enum TextOrArray {
    Text(Vec<u8>),
    Array(Vec<TextArrayElement>),
}

pub struct TextObject {
    pub font: Name,
    pub font_size: f32,
    // matrix: [f32; 6],
    /// A "simplified" text matrix.
    pub position: (f32, f32),
    pub parameters: TextStateParameters,
    // FIXME: this indirection will be needed down the line. For now it's seems a bit dumb.
    // It should be replaced with a `VecDeque<u8>` to allow the *font* to iterate over the text
    pub text_buffer: Option<PDFString>,
    pub buffer: Option<VecDeque<TextArrayElement>>,
}

impl TextObject {
    pub fn move_to(&mut self, x: f32, y: f32) {
        self.position = (x, y);
    }
    pub fn move_to_next_line(&mut self) {
        self.position.1 -= self.parameters.leading;
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

pub struct TextObjectStream<Ops> {
    text_object: TextObject,
    ops: Ops,
}

impl<Ops> TextObjectStream<Ops>
where
    Ops: Iterator<Item = Operator>,
{
    fn build(mut ops: Ops) -> Self {
        let mut position = (0.0, 0.0);
        let mut parameters = Default::default();

        for operator in &mut ops {
            match operator {
                Operator::Text(TextOperator::TextStateOperator(
                    TextStateOperator::SetFontAndFontSize(SetFontAndFontSize(font, font_size)),
                )) => {
                    let text_object = TextObject {
                        font,
                        font_size,
                        position,
                        parameters,
                        text_buffer: None,
                        buffer: None,
                    };

                    return TextObjectStream { text_object, ops };
                }
                Operator::Text(TextOperator::TextStateOperator(op)) => {
                    op.preapply(&mut position, &mut parameters);
                }
                Operator::Text(TextOperator::TextPositioningOperator(op)) => {
                    op.preapply(&mut position, &mut parameters);
                }
                Operator::Text(TextOperator::TextShowingOperator(op)) => {
                    unreachable!(
                        "Trying to show text before setting the font and font size: {op:?}"
                    );
                }
                _ => {
                    // FIXME: Use proper logging.
                    debug!("Skipping operator: {:?}", operator);
                }
            }
        }

        // FIXME: replace these `unreachable!` with proper error handling
        unreachable!("A `BT` tag was found, but the text object was not completed");
    }
}

pub fn parse_text_object<Ops>(mut ops: Ops) -> Option<TextObjectStream<Ops>>
where
    Ops: Iterator<Item = Operator>,
{
    while let Some(op) = ops.next() {
        match op {
            Operator::BeginText(_) => return Some(TextObjectStream::build(ops)),
            _ => {
                // NOTE: just skip any other operators until we find the text object
            }
        }
    }
    None
}

impl Iterator for TextObject {
    type Item = ((f32, f32), PDFString);

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
                        self.position.0 += offset;
                    }
                }
            }
        }

        let text = self.text_buffer.take()?;

        Some((self.position, text))
    }
}

impl<Ops> Iterator for TextObjectStream<Ops>
where
    Ops: Iterator<Item = Operator>,
{
    type Item = ((f32, f32), PDFString);

    fn next(&mut self) -> Option<Self::Item> {
        while self.text_object.buffer.is_none() {
            let op = self.ops.next()?;
            match op {
                Operator::EndText(_) => return None,
                Operator::Text(op) => op.apply(&mut self.text_object),
                _ => {
                    debug!("Skipping operator: {:?}", op);
                }
            }
        }

        self.text_object.next()
    }
}
