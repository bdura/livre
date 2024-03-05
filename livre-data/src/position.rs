use livre_extraction::{extract, Extract};
use livre_utilities::space;
use nom::sequence::separated_pair;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

impl Extract<'_> for Position {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        let (input, (x, y)) = separated_pair(extract, space, extract)(input)?;
        Ok((input, Position { x, y }))
    }
}

impl Position {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(b"0 0", Position::new(0.0, 0.0))]
    #[case(b"-10 0", Position::new(-10.0, 0.0))]
    #[case(b"-10.5 0.5", Position::new(-10.5, 0.5))]
    fn point(#[case] input: &[u8], #[case] expected: Position) {
        let (_, point) = extract(input).unwrap();
        assert_eq!(expected, point);
    }
}
