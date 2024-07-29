use crate::parsers::take_whitespace1;
use crate::parsers::{extract, Extract, Name};
use crate::text::TextState;
use nom::{bytes::complete::tag, sequence::preceded};

use super::super::super::operators::Operator;

#[derive(Debug, PartialEq, Clone)]
pub struct FontSize {
    pub font: String,
    pub size: f32,
}

impl Extract<'_> for FontSize {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        let (input, (Name(font), size)) = extract(input)?;
        let (input, _) = preceded(take_whitespace1, tag("Tf"))(input)?;

        let font_size = FontSize { font, size };
        Ok((input, font_size))
    }
}

impl FontSize {
    pub fn new(font: impl Into<String>, size: f32) -> Self {
        let font = font.into();
        Self { font, size }
    }
}

impl Operator for FontSize {
    fn apply(self, obj: &mut TextState) {
        let FontSize { font, size } = self;
        obj.font = font;
        obj.size = size;
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(b"/F1 10 Tf", FontSize::new("F1", 10.0))]
    #[case(b"/F2 9.0 Tf", FontSize::new("F2", 9.0))]
    fn font_size(#[case] input: &[u8], #[case] expected: FontSize) {
        let (_, fs) = FontSize::extract(input).unwrap();
        assert_eq!(fs, expected);
    }
}
