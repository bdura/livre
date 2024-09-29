use std::collections::VecDeque;

use winnow::{
    combinator::{alt, trace},
    BStr, PResult, Parser,
};

use crate::Extract;

#[derive(Debug, PartialEq, Clone)]
pub struct MaybeArray<T>(pub Vec<T>);

impl<T> Default for MaybeArray<T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<T> From<MaybeArray<T>> for Vec<T> {
    fn from(value: MaybeArray<T>) -> Self {
        value.0
    }
}

impl<T> From<MaybeArray<T>> for VecDeque<T> {
    fn from(value: MaybeArray<T>) -> Self {
        value.0.into()
    }
}

impl<T> From<Vec<T>> for MaybeArray<T> {
    fn from(value: Vec<T>) -> Self {
        Self(value)
    }
}

impl<'de, T> Extract<'de> for MaybeArray<T>
where
    T: Extract<'de>,
{
    fn extract(input: &mut &'de BStr) -> PResult<Self> {
        trace(
            "livre-maybe-array",
            alt((T::extract.map(|t| vec![t]), Vec::<T>::extract)).map(Self),
        )
        .parse_next(input)
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use super::*;

    use rstest::rstest;

    #[rstest]
    #[case(b"[true true]", vec![true, true])]
    #[case(b"true", vec![true])]
    #[case(b"1", vec![1.0f64])]
    fn maybe_array<'de, T>(#[case] input: &'de [u8], #[case] expected: Vec<T>)
    where
        T: Extract<'de> + Debug + PartialEq,
    {
        let result = MaybeArray::<T>::extract(&mut input.as_ref()).unwrap();
        assert_eq!(result, expected.into());
    }
}
