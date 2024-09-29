use winnow::{combinator::trace, BStr, PResult, Parser};

use crate::{extraction::extract, Extract};

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Rectangle {
    pub xmin: f32,
    pub ymin: f32,
    pub xmax: f32,
    pub ymax: f32,
}

impl From<(f32, f32, f32, f32)> for Rectangle {
    fn from((xmin, ymin, xmax, ymax): (f32, f32, f32, f32)) -> Self {
        Self {
            xmin,
            ymin,
            xmax,
            ymax,
        }
    }
}

impl Extract<'_> for Rectangle {
    fn extract(input: &mut &BStr) -> PResult<Self> {
        trace("livre-rectangle", move |i: &mut &BStr| {
            let [xmin, ymin, xmax, ymax] = extract(i)?;
            Ok(Self {
                xmin,
                ymin,
                xmax,
                ymax,
            })
        })
        .parse_next(input)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(b"[ 0 0 10 10]", Rectangle::from((0.0, 0.0, 10.0, 10.0)))]
    #[case(b"[ -3 2 10 10.5]", Rectangle::from((-3.0, 2.0, 10.0, 10.5)))]
    fn rectangle(#[case] input: &[u8], #[case] expected: Rectangle) {
        let res = extract(&mut input.as_ref()).unwrap();
        assert_eq!(expected, res);
    }
}
