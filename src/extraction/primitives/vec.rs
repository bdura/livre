use winnow::ascii::multispace0;
use winnow::combinator::{delimited, preceded, repeat, trace};
use winnow::{BStr, PResult, Parser};

use super::Extract;

impl<'de, T> Extract<'de> for Vec<T>
where
    T: Extract<'de>,
{
    fn extract(input: &mut &'de BStr) -> PResult<Self> {
        trace(
            "livre-vec",
            delimited(
                b'[',
                repeat(0.., preceded(multispace0, T::extract)),
                (multispace0, b']'),
            ),
        )
        .parse_next(input)
    }
}
#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use rstest::rstest;

    use crate::{extraction::extract, Extract};

    #[rstest]
    #[case(b"[true true  false]", vec![true, true, false])]
    #[case(b"[  true true  false  ]", vec![true, true, false])]
    fn vec<T>(#[case] input: &[u8], #[case] expected: T)
    where
        T: for<'a> Extract<'a> + Debug + PartialEq,
    {
        let res = extract(&mut input.as_ref()).unwrap();
        assert_eq!(expected, res);
    }
}
