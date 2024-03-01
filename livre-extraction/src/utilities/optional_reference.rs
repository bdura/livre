use nom::{branch::alt, combinator::map, IResult};

use crate::{Extract, Reference};

#[derive(Debug, PartialEq, Clone)]
pub enum OptRef<T> {
    Val(T),
    Ref(Reference),
}

impl<'input, T> Extract<'input> for OptRef<T>
where
    T: Extract<'input>,
{
    fn extract(input: &'input [u8]) -> IResult<&'input [u8], Self> {
        alt((
            map(Reference::extract, Self::Ref),
            map(T::extract, Self::Val),
        ))(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(b"0 1 R", OptRef::Ref(Reference::new(0, 1)))]
    #[case(b"-12", OptRef::Val(-12))]
    #[case(b"10 1 R", OptRef::Ref(Reference::new(10, 1)))]
    fn maybe_array_i32(#[case] input: &[u8], #[case] expected: OptRef<i32>) {
        let (_, opt_ref) = OptRef::<i32>::extract(input).unwrap();
        assert_eq!(opt_ref, expected);
    }
}
