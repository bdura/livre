use std::fmt::Debug;

use crate::{
    data::Rectangle,
    parsers::{take_whitespace1, Extract},
};
use nalgebra::Matrix3;
use nom::{
    bytes::complete::{take, take_until},
    sequence::terminated,
    IResult,
};

use super::{
    operators::{FontSize, RenderMode},
    Op, Operator,
};

#[derive(Debug, PartialEq, Clone)]
pub struct TextElement {
    pub text: String,
    pub bounding_box: Rectangle,
}

#[derive(Debug)]
pub struct TextState {
    pub character_spacing: f32,
    pub word_spacing: f32,
    pub horizontal_scaling: f32,
    pub leading: f32,
    pub font: String,
    pub size: f32,
    pub mode: RenderMode,
    pub rise: f32,
    pub text_matrix: Matrix3<f32>,
    pub text_line_matrix: Matrix3<f32>,
    // pub elements: Vec<TextElement>,
}

impl TextState {
    pub fn new(font: String, size: f32) -> Self {
        Self {
            character_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 1.0,
            leading: 0.0,
            font,
            size,
            mode: RenderMode::Fill,
            rise: 0.0,
            text_matrix: Matrix3::identity(),
            text_line_matrix: Matrix3::identity(),
        }
    }

    pub(crate) fn apply<O: Operator>(&mut self, op: O) {
        op.apply(self)
    }

    /// `Td` operator.
    ///
    /// Move to the start of the next line, offset from the start of the
    /// current line by `(x, y)`. `x` and `y` are expressed in unscaled text space units.
    pub fn translate(&mut self, x: f32, y: f32) {
        let m = Matrix3::new(1.0, 0.0, 0.0, 0.0, 1.0, 0.0, x, y, 1.0);
        self.text_matrix = m * self.text_line_matrix;
        self.text_line_matrix = self.text_matrix;
    }

    pub fn offset(&mut self, amount: f32) {
        // TODO: handle vertical/horizontal
        let m = Matrix3::new(1.0, 0.0, 0.0, 0.0, 1.0, 0.0, amount, 0.0, 1.0);
        self.text_matrix = m * self.text_line_matrix;
        self.text_line_matrix = self.text_matrix;
    }

    /// `TL` operator
    pub fn set_leading(&mut self, leading: f32) {
        self.leading = leading;
    }

    /// `TD` operator
    ///
    /// Move to the start of the next line, offset from the start of the
    /// current line by `(x, y)``. As a side effect, sets the leading parameter
    /// in the text state. This operator has the same effect as this code:
    ///
    /// ```no-rust
    /// -y TL
    /// x y Td
    /// ````
    pub fn translate_and_set_leading(&mut self, x: f32, y: f32) {
        self.set_leading(-y);
        self.translate(x, y);
    }

    /// `Tm` operator
    pub fn set_matrix(&mut self, a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) {
        let m = Matrix3::new(a, b, 0.0, c, d, 0.0, e, f, 1.0);

        self.text_matrix = m;
        self.text_line_matrix = m;
    }

    /// `T*` operator
    pub(crate) fn next_line(&mut self) {
        self.translate_and_set_leading(0.0, -self.leading);
    }

    // fn add_text_element(&mut self, element: TextElement) {
    //     self.elements.push(element);
    // }

    /// Tj operator
    pub(crate) fn show_text(&mut self, text: String) {
        // TODO: create text element, add
        eprintln!("Trying to show `{text}`");
        todo!()
    }

    pub(crate) fn set_character_spacing(&mut self, spacing: f32) {
        self.character_spacing = spacing;
    }
    pub(crate) fn set_word_spacing(&mut self, spacing: f32) {
        self.word_spacing = spacing;
    }
}

pub struct ObjectContent<'a>(&'a [u8]);

impl<'a> Debug for ObjectContent<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ObjectContent")
            .field(&String::from_utf8_lossy(self.0))
            .finish()
    }
}

impl<'a> Iterator for ObjectContent<'a> {
    type Item = Op;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((input, line)) = take_line(self.0).ok() {
            self.0 = input;

            if let Ok((_, op)) = Op::extract(line) {
                return Some(op);
            }
        }
        None
    }
}

#[derive(Debug)]
pub struct TextObject<'a> {
    pub content: ObjectContent<'a>,
    pub state: TextState,
}

pub struct TextObjectIterator<'a> {
    input: &'a [u8],
}

impl<'a> TextObjectIterator<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self { input }
    }
}

impl<'a> Iterator for TextObjectIterator<'a> {
    type Item = TextObject<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let (input, content) = find_next_object(self.input).ok()?;
        self.input = input;

        let mut content = ObjectContent(content);

        let op = content.next()?;

        if let Op::FontSize(FontSize { font, size }) = op {
            Some(TextObject {
                content,
                state: TextState::new(font, size),
            })
        } else {
            panic!("Text object should define font & size before anything else.");
        }
    }
}

fn find_next_object(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (input, _) = take_until("BT")(input)?;
    let (input, _) = take(2usize)(input)?;
    let (input, _) = take_whitespace1(input)?;
    take_until("ET")(input)
}

fn take_line(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (input, line) = take_until("\n")(input)?;
    let (input, _) = take(1usize)(input)?;

    let line = line.strip_suffix(b"\r").unwrap_or(line);

    Ok((input, line))
}
