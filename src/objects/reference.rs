use nom::{bytes::complete::tag, character::complete::digit1, sequence::tuple, IResult};

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
        let (input, (obj, _, gen, _)) = tuple((digit1, tag(b" "), digit1, tag(b" R")))(input)?;

        // SAFETY: obj is guaranteed to contain digits
        let object: usize = unsafe { std::str::from_utf8_unchecked(obj).parse().unwrap() };
        // SAFETY: gen is guaranteed to contain digits
        let generation: u16 = unsafe { std::str::from_utf8_unchecked(gen).parse().unwrap() };

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
