use nom::{branch::alt, combinator::map, IResult};

use crate::Extract;

#[derive(Debug, PartialEq, Clone)]
pub struct MaybeArray<T>(pub Vec<T>);

impl<'input, T> Extract<'input> for MaybeArray<T>
where
    T: Extract<'input>,
{
    fn extract(input: &'input [u8]) -> IResult<&'input [u8], Self> {
        let (input, value) = alt((Vec::<T>::extract, map(T::extract, |r| vec![r])))(input)?;
        Ok((input, Self(value)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(b"[1 2 3]", &[1, 2, 3])]
    #[case(b"1", &[1])]
    fn maybe_array_i32(#[case] input: &[u8], #[case] expected: &[i32]) {
        let (_, MaybeArray(array)) = MaybeArray::<i32>::extract(input).unwrap();
        assert_eq!(array, expected);
    }
}
