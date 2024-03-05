use livre_extraction::{extract, Extract};

use crate::Point;

#[derive(Debug, PartialEq, Clone)]
pub struct Rectangle {
    pub lower_left: Point,
    pub upper_right: Point,
}

impl Extract<'_> for Rectangle {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        let (input, [lower_left, upper_right]) = extract(input)?;
        Ok((
            input,
            Self {
                lower_left,
                upper_right,
            },
        ))
    }
}

impl Rectangle {
    pub fn from_ll_ur(llx: i32, lly: i32, urx: i32, ury: i32) -> Self {
        let lower_left = Point::new(llx, lly);
        let upper_right = Point::new(urx, ury);
        Self {
            lower_left,
            upper_right,
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(b"[ 0 0 10 10]", Rectangle::from_ll_ur(0, 0, 10, 10))]
    #[case(b"[ -3 2 10 10]", Rectangle::from_ll_ur(-3, 2, 10, 10))]
    fn rectangle(#[case] input: &[u8], #[case] expected: Rectangle) {
        let (_, rectangle) = extract(input).unwrap();
        assert_eq!(expected, rectangle);
    }
}
