use livre_extraction::Extract;
use livre_utilities::take_whitespace1;
use nom::{bytes::complete::tag, sequence::tuple};

use crate::operators::Operator;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum RenderMode {
    Fill,
    Stroke,
    FillThenStroke,
    Invisible,
    FillAndClip,
    StrokeAndClip,
    FillThenStrokeAndClip,
    Clip,
}

impl Extract<'_> for RenderMode {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        let (input, render) = u8::extract(input)?;
        let (input, _) = tuple((take_whitespace1, tag(b"Tr")))(input)?;

        let mode = match render {
            0 => Self::Fill,
            1 => Self::Stroke,
            2 => Self::FillThenStroke,
            3 => Self::Invisible,
            4 => Self::FillAndClip,
            5 => Self::StrokeAndClip,
            6 => Self::FillThenStrokeAndClip,
            7 => Self::Clip,
            _ => unreachable!("Per the specs."),
        };

        Ok((input, mode))
    }
}

impl Operator for RenderMode {
    fn apply(self, obj: &mut crate::TextState) {
        obj.mode = self;
    }
}

#[cfg(test)]
mod tests {
    use livre_extraction::extract;
    use rstest::rstest;

    use super::*;
    use RenderMode::*;

    #[rstest]
    #[case(b"0 Tr", Fill)]
    fn render_mode(#[case] input: &[u8], #[case] expected: RenderMode) {
        let (_, mode) = extract(input).unwrap();
        assert_eq!(expected, mode);
    }
}
