use std::{collections::HashMap, fmt::Debug};

use crate::{
    data::{Position, Rectangle},
    fonts::{Font, FontBehavior},
    objects::Bytes,
    parsers::{extract, take_whitespace, take_whitespace1, Extract},
    structure::BuiltPage,
};
use nalgebra::Matrix3;
use nom::{
    bytes::complete::{tag, take, take_until},
    combinator::recognize,
    multi::many1,
    sequence::{preceded, tuple},
    IResult,
};

use super::{
    operators::{FontSize, PdfString, RenderMode},
    Op, Operator,
};

#[derive(Debug, PartialEq, Clone)]
pub struct TextElement {
    pub char: char,
    pub bounding_box: Rectangle,
}

#[derive(Debug)]
pub struct TextState<'a> {
    pub font_name: String,
    pub font: &'a Font,
    pub size: f32,
    pub character_spacing: f32,
    pub word_spacing: f32,
    pub horizontal_scaling: f32,
    pub leading: f32,
    pub mode: RenderMode,
    pub rise: f32,
    pub text_matrix: Matrix3<f32>,
    pub text_line_matrix: Matrix3<f32>,
    pub elements: Vec<TextElement>,
}

impl<'a> TextState<'a> {
    pub fn new(font_name: String, font: &'a Font, size: f32) -> Self {
        Self {
            font_name,
            font,
            size,
            character_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 1.0,
            leading: 0.0,
            mode: RenderMode::Fill,
            rise: 0.0,
            text_matrix: Matrix3::identity(),
            text_line_matrix: Matrix3::identity(),
            elements: Vec::new(),
        }
    }

    pub fn apply<O: Operator>(&mut self, op: O) {
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

    pub fn offset_tj(&mut self, amount: f32) {
        let offset = -amount * self.horizontal_scaling * self.size;
        // TODO: handle vertical/horizontal
        let m = Matrix3::new(1.0, 0.0, 0.0, 0.0, 1.0, 0.0, offset, 0.0, 1.0);
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

    pub(crate) fn start_position(&self) -> Position {
        Position::new(
            self.text_matrix[(2, 0)],
            self.text_matrix[(2, 1)] + self.font.descent() * self.size,
        )
    }

    pub(crate) fn end_position(&self) -> Position {
        Position::new(
            self.text_matrix[(2, 0)],
            self.text_matrix[(2, 1)] + self.font.ascent() * self.size,
        )
    }

    /// Tj operator
    pub(crate) fn show_text(&mut self, input: PdfString) {
        // TODO: create text element

        for (char, width, is_space) in self.font.process(input) {
            let start_position = self.start_position();

            let mut tx = width * self.size + self.character_spacing;

            if is_space {
                tx += self.word_spacing;
            }

            self.translate(tx * self.horizontal_scaling, 0.0);

            self.elements.push(TextElement {
                char,
                bounding_box: Rectangle {
                    lower_left: start_position,
                    upper_right: self.end_position(),
                },
            })
        }
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
        let (input, _) = take_whitespace(self.0).ok()?;
        let (input, op) = extract(input).map_err(|e| e.map_input(Bytes::from)).ok()?;
        self.0 = input;
        Some(op)
    }
}

#[derive(Debug)]
pub struct TextObject<'a> {
    pub content: ObjectContent<'a>,
    pub state: TextState<'a>,
}

pub struct TextObjectIterator<'a> {
    input: &'a [u8],
    fonts: &'a HashMap<String, Font>,
}

impl<'a> From<&'a BuiltPage> for TextObjectIterator<'a> {
    fn from(page: &'a BuiltPage) -> Self {
        TextObjectIterator::new(&page.content, &page.fonts)
    }
}

impl<'a> TextObjectIterator<'a> {
    pub fn new(input: &'a [u8], fonts: &'a HashMap<String, Font>) -> Self {
        Self { input, fonts }
    }
}

impl<'a> Iterator for TextObjectIterator<'a> {
    type Item = TextObject<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let (input, content) = find_next_object(self.input).ok()?;
        self.input = input;

        let mut content = ObjectContent(content);

        let op = content.next()?;

        if let Op::FontSize(FontSize { font_name, size }) = op {
            let font = self.fonts.get(&font_name).unwrap();
            Some(TextObject {
                content,
                state: TextState::new(font_name, font, size),
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

    let (input, object) = recognize(many1(preceded(take_whitespace, Op::extract)))(input)?;
    let (input, _) = tuple((take_whitespace, tag(b"ET")))(input)?;

    Ok((input, object))
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(
        indoc!{b"
            BT
            /F1 8 Tf
            1 0 0 1 9.84 612.34 Tm
            0.2 g
            0.2 G
            [(9)-6(4)-6(0)7(1)-6(0)7( )-2(CRET)-3(EIL)8( )-2(Ce)4(d)-6(e)4(x)] TJ
            ET
        "},
    )]
    fn text_object(#[case] input: &[u8]) {
        let (_, object) = find_next_object(input).unwrap();

        println!("{:?}", Bytes::from(object));

        assert!(object.starts_with(b"/F1"));
        assert!(object.ends_with(b"] TJ"));

        assert_eq!(object, &input[3..(input.len() - 4)]);
    }
}
