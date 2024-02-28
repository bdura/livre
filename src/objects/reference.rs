use nom::{bytes::complete::tag, sequence::tuple, IResult};

use crate::utilities::parse_digits;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct Reference {
    pub object: usize,
    pub generation: u16,
}

impl Reference {
    pub fn new(object: usize, generation: u16) -> Self {
        Self { object, generation }
    }
}

impl Reference {
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, (object, _, generation, _)) =
            tuple((parse_digits, tag(" "), parse_digits, tag(" R")))(input)?;

        Ok((input, Self { object, generation }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn parse(input: &[u8]) -> Reference {
        let (_, obj) = Reference::parse(input).unwrap();
        obj
    }

    #[allow(clippy::bool_assert_comparison)]
    #[rstest]
    #[case(b"1 0 R", Reference::new(1, 0))]
    #[case(b"10 33 R", Reference::new(10, 33))]
    fn test_parse(#[case] input: &[u8], #[case] result: Reference) {
        assert_eq!(parse(input), result);
    }
}
