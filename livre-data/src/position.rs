use livre_extraction::Extract;
use livre_serde::extract_deserialize;
use serde::Deserialize;

#[derive(Debug, PartialEq, Clone, Copy, Deserialize)]
#[serde(from = "(f32, f32)")]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

impl From<(f32, f32)> for Position {
    fn from((x, y): (f32, f32)) -> Self {
        Self { x, y }
    }
}

impl Extract<'_> for Position {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        extract_deserialize(input)
    }
}

impl Position {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[cfg(test)]
mod tests {
    use livre_extraction::extract;
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
