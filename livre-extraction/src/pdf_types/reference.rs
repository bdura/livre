use nom::{bytes::complete::tag, sequence::tuple, IResult};

use crate::Extract;

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

impl Extract<'_> for Reference {
    fn extract(input: &'_ [u8]) -> IResult<&'_ [u8], Self> {
        let (input, (object, _, generation, _)) =
            tuple((usize::extract, tag(" "), u16::extract, tag(" R")))(input)?;

        Ok((input, Self { object, generation }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(b"1 0 R", Reference::new(1, 0))]
    #[case(b"10 33 R", Reference::new(10, 33))]
    fn reference(#[case] input: &[u8], #[case] result: Reference) {
        let (_, reference) = Reference::extract(input).unwrap();
        assert_eq!(reference, result);
    }
}
