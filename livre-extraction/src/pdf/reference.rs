use std::marker::PhantomData;

use nom::{bytes::complete::tag, sequence::tuple, IResult};

use crate::{extract, Extract};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct Reference {
    pub object: usize,
    pub generation: u16,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct TypedReference<T> {
    pub reference: Reference,
    marker: PhantomData<T>,
}

impl Reference {
    pub fn new(object: usize, generation: u16) -> Self {
        Self { object, generation }
    }

    pub fn first(object: usize) -> Self {
        Self {
            object,
            generation: 0,
        }
    }
}

impl<T> TypedReference<T> {
    pub fn new(object: usize, generation: u16) -> Self {
        Self {
            reference: Reference::new(object, generation),
            marker: PhantomData,
        }
    }
}

impl Extract<'_> for Reference {
    fn extract(input: &'_ [u8]) -> IResult<&'_ [u8], Self> {
        let (input, (object, _, generation, _)) =
            tuple((usize::extract, tag(" "), u16::extract, tag(" R")))(input)?;

        Ok((input, Self { object, generation }))
    }
}

impl<'input, T> Extract<'input> for TypedReference<T>
where
    T: Extract<'input>,
{
    fn extract(input: &'_ [u8]) -> IResult<&'_ [u8], Self> {
        let (input, (object, _, generation, _)) =
            tuple((extract, tag(" "), extract, tag(" R")))(input)?;

        Ok((input, Self::new(object, generation)))
    }
}

#[cfg(test)]
mod tests {
    use crate::NoOp;

    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(b"1 0 R", Reference::new(1, 0))]
    #[case(b"10 33 R", Reference::new(10, 33))]
    fn reference(#[case] input: &[u8], #[case] result: Reference) {
        let (_, reference) = Reference::extract(input).unwrap();
        assert_eq!(reference, result);
    }

    #[rstest]
    #[case(b"1 0 R", TypedReference::new(1, 0))]
    #[case(b"10 33 R", TypedReference::new(10, 33))]
    fn noop_typed_reference(#[case] input: &[u8], #[case] expected: TypedReference<NoOp>) {
        let (_, reference) = extract(input).unwrap();
        assert_eq!(expected, reference);
    }
}
