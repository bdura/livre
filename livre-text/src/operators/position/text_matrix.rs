use livre_extraction::{extract, Extract};
use nalgebra::Matrix3;

use crate::Operator;

#[derive()]
pub struct TextMatrix {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub f: f32,
}

impl Extract<'_> for TextMatrix {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        let (input, (a, b, c, d, e, f)) = extract(input)?;
        let matrix = Self { a, b, c, d, e, f };
        Ok((input, matrix))
    }
}

impl Operator for TextMatrix {
    fn operate(self, obj: &mut crate::TextObject) {
        let TextMatrix { a, b, c, d, e, f } = self;
        obj.set_matrix(a, b, c, d, e, f);
    }
}
