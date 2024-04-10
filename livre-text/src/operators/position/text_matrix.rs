use livre_extraction::{extract, Extract};
use nalgebra::Matrix3;

use crate::Operator;

#[derive(Debug, PartialEq, Clone)]
pub struct TextMatrix {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub f: f32,
}

impl From<TextMatrix> for Matrix3<f32> {
    fn from(value: TextMatrix) -> Self {
        let TextMatrix { a, b, c, d, e, f } = value;
        Matrix3::new(a, b, 0.0, c, d, 0.0, e, f, 1.0)
    }
}

impl Extract<'_> for TextMatrix {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        let (input, (a, b, c, d, e, f)) = extract(input)?;
        let matrix = Self { a, b, c, d, e, f };
        Ok((input, matrix))
    }
}

impl Operator for TextMatrix {
    fn apply(self, obj: &mut crate::TextState) {
        let TextMatrix { a, b, c, d, e, f } = self;
        obj.set_matrix(a, b, c, d, e, f);
    }
}
