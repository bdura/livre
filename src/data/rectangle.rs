use crate::parsers::Extract;
use crate::serde::extract_deserialize;
use serde::Deserialize;

use super::Position;

#[derive(Debug, PartialEq, Clone, Deserialize)]
#[serde(from = "[f32;4]")]
pub struct Rectangle {
    pub lower_left: Position,
    pub upper_right: Position,
}

impl From<[Position; 2]> for Rectangle {
    fn from([lower_left, upper_right]: [Position; 2]) -> Self {
        Self {
            lower_left,
            upper_right,
        }
    }
}

impl From<[f32; 4]> for Rectangle {
    fn from([llx, lly, urx, ury]: [f32; 4]) -> Self {
        Self::from_ll_ur(llx, lly, urx, ury)
    }
}

impl Extract<'_> for Rectangle {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        extract_deserialize(input)
    }
}

impl Rectangle {
    pub fn from_ll_ur(llx: f32, lly: f32, urx: f32, ury: f32) -> Self {
        let lower_left = Position::new(llx, lly);
        let upper_right = Position::new(urx, ury);
        Self {
            lower_left,
            upper_right,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::serde::from_bytes;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(b"[ 0 0 10 10]", Rectangle::from_ll_ur(0.0, 0.0, 10.0, 10.0))]
    #[case(b"[ -3 2 10 10.5]", Rectangle::from_ll_ur(-3.0, 2.0, 10.0, 10.5))]
    fn rectangle(#[case] input: &[u8], #[case] expected: Rectangle) {
        assert_eq!(expected, from_bytes(input).unwrap());
        // let (_, rectangle) = extract(input).unwrap();
        // assert_eq!(expected, rectangle);
    }
}
