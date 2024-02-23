use nom::{bytes::complete::tag, character::complete::digit1, sequence::tuple, IResult};

#[derive(Debug, PartialEq, Clone, Copy, Hash)]
pub struct Reference {
    pub(crate) object: usize,
    pub(crate) generation: usize,
}

impl Reference {
    pub(crate) fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, (obj, _, gen, _)) = tuple((digit1, tag(b" "), digit1, tag(b" R")))(input)?;

        // SAFETY: obj is guaranteed to contain digits
        let object: usize = unsafe { std::str::from_utf8_unchecked(obj).parse().unwrap() };
        // SAFETY: gen is guaranteed to contain digits
        let generation: usize = unsafe { std::str::from_utf8_unchecked(gen).parse().unwrap() };

        Ok((input, Self { object, generation }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn new_ref(object: usize, generation: usize) -> Reference {
        Reference { object, generation }
    }

    fn parse(input: &[u8]) -> Reference {
        let (_, obj) = Reference::parse(input).unwrap();
        obj
    }

    #[allow(clippy::bool_assert_comparison)]
    #[rstest]
    #[case(b"1 0 R", new_ref(1, 0))]
    #[case(b"10 33 R", new_ref(10, 33))]
    fn test_parse(#[case] input: &[u8], #[case] result: Reference) {
        assert_eq!(parse(input), result);
    }
}
