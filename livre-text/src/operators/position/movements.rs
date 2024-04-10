use livre_extraction::{extract, Extract};
use livre_utilities::take_whitespace1;
use nom::{bytes::complete::tag, combinator::value, sequence::tuple};

use crate::Operator;

///`Td` operator: move to the start of the next line,
/// offset from the start of the current line by (`x`, `y`).
/// `x` and `y` shall denote numbers expressed in unscaled text space units.
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct MoveTd {
    pub x: f32,
    pub y: f32,
}

/// `TD` operator: move to the start of the next line,
/// offset from the start of the current line by (`x`, `y`).
/// As a side effect, this operator shall set the leading parameter in the text state
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct MoveTD {
    pub x: f32,
    pub y: f32,
}

/// `T*` operator: move to the start of the next line.
///
/// This operator has the same effect as the code 0 –Tl TD
/// where Tl denotes the current leading parameter in the text state.
/// The negative of Tl is used here because Tl is the text leading
/// expressed as a positive number.
/// Going to the next line entails decreasing the y coordinate.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct MoveNextLine;

impl Extract<'_> for MoveNextLine {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        value(Self, tag(b"T*"))(input)
    }
}

macro_rules! impl_extract {
    ($ty:ident + $tag:literal) => {
        impl Extract<'_> for $ty {
            fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
                let (input, (x, y)) = extract(input)?;
                let (input, _) = tuple((take_whitespace1, tag($tag)))(input)?;

                let m = Self { x, y };

                Ok((input, m))
            }
        }
    };
}

impl_extract!(MoveTd + b"Td");
impl_extract!(MoveTD + b"TD");

impl Operator for MoveTd {
    fn apply(self, obj: &mut crate::TextState) {
        let Self { x, y } = self;
        obj.translate(x, y);
    }
}

impl Operator for MoveTD {
    fn apply(self, obj: &mut crate::TextState) {
        let Self { x, y } = self;
        obj.translate_and_set_leading(x, y);
    }
}

impl Operator for MoveNextLine {
    fn apply(self, obj: &mut crate::TextState) {
        obj.next_line();
    }
}
