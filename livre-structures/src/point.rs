use livre_extraction::{extract, Extract};
use livre_utilities::space;
use nom::sequence::separated_pair;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Extract<'_> for Point {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        let (input, (x, y)) = separated_pair(extract, space, extract)(input)?;
        Ok((input, Point { x, y }))
    }
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(b"0. 0.", Point::new(0.0, 0.0))]
    #[case(b"-10. 0.", Point::new(-10.0, 0.0))]
    fn point(#[case] input: &[u8], #[case] expected: Point) {
        let (_, point) = extract(input).unwrap();
        assert_eq!(expected, point);
    }
}