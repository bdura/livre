use winnow::ascii::multispace0;
use winnow::combinator::{delimited, preceded, repeat, trace};
use winnow::{BStr, PResult, Parser};

use super::Extract;

/// From the specification:
///
/// > An array shall be written as a sequence of objects enclosed in SQUARE BRACKETS
/// > (using LEFT SQUARE BRACKET (5Bh) and RIGHT SQUARE BRACKET (5Dh)).
/// >
/// > PDF syntax directly supports only one-dimensional arrays. Arrays of higher dimension
/// > can be constructed by using arrays as elements of arrays, nested to any depth.
///
/// ## Example
///
/// ```raw
/// [549 3.14 false (Ralph) /SomeName]
/// ```
///
/// Note that in Livre, arrays have homogeneous type; however you may require an array of
/// [`Object`](crate::extraction::Object).
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

    use crate::extraction::{extract, Extract};

    #[rstest]
    #[case(b"[true true  false]", vec![true, true, false])]
    #[case(b"[  true true  false  ]", vec![true, true, false])]
    #[case(b"[  1   2 3]", vec![1, 2, 3])]
    #[case(b"[ 1 2 3]", vec![1u8, 2u8, 3u8])]
    fn vec<T>(#[case] input: &[u8], #[case] expected: T)
    where
        T: for<'a> Extract<'a> + Debug + PartialEq,
    {
        let res = extract(&mut input.as_ref()).unwrap();
        assert_eq!(expected, res);
    }
}
