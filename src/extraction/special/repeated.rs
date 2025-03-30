use std::collections::VecDeque;

use winnow::{
    ascii::multispace0,
    combinator::{repeat, terminated, trace},
    BStr, PResult, Parser,
};

use crate::extraction::Extract;

/// Collects a sequence of repeated elements into a [`Vec`].
/// Contrary to the vanilla [`Vec`] extractor, `Repeated` does **not** match
/// on opening and closing brackets.
#[derive(Debug, PartialEq, Clone)]
pub struct Repeated<T>(pub Vec<T>);

impl<T> From<Repeated<T>> for Vec<T> {
    fn from(value: Repeated<T>) -> Self {
        value.0
    }
}

impl<T> From<Repeated<T>> for VecDeque<T> {
    fn from(value: Repeated<T>) -> Self {
        value.0.into()
    }
}

impl<T> From<Vec<T>> for Repeated<T> {
    fn from(value: Vec<T>) -> Self {
        Self(value)
    }
}

impl<'de, T> Extract<'de> for Repeated<T>
where
    T: Extract<'de>,
{
    fn extract(input: &mut &'de BStr) -> PResult<Self> {
        trace(
            "livre-repeated",
            repeat(0.., terminated(T::extract, multispace0)).map(Self),
        )
        .parse_next(input)
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use crate::extraction::extract;

    use super::*;

    use rstest::rstest;

    #[rstest]
    #[case(b"true true", vec![true, true])]
    #[case(b"true", vec![true])]
    #[case(b"1", vec![1.0f64])]
    fn repeated<'de, T>(#[case] input: &'de [u8], #[case] expected: Vec<T>)
    where
        T: Extract<'de> + Debug + PartialEq,
    {
        let Repeated(inner) = extract(&mut input.as_ref()).unwrap();
        assert_eq!(expected, inner);
    }
}
