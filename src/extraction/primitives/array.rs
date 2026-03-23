use winnow::{
    ascii::multispace0,
    combinator::{delimited, preceded, repeat, trace},
    error::{ContextError, ErrMode},
    BStr, PResult, Parser,
};

use crate::extraction::Extract;

impl<'de, T, const N: usize> Extract<'de> for [T; N]
where
    T: Extract<'de>,
{
    fn extract(input: &mut &'de BStr) -> PResult<Self> {
        trace("livre-array", move |i: &mut &'de BStr| {
            let vec: Vec<T> = delimited(
                b'[',
                repeat(0.., preceded(multispace0, T::extract)),
                (multispace0, b']'),
            )
            .parse_next(i)?;

            // Convert the collected `Vec` into a fixed-size array. `TryFrom<Vec<T>> for [T; N]`
            // (stable since Rust 1.59) checks that `vec.len() == N` and returns `Err(vec)` if
            // not. We map that failure to a backtrack error so the caller can try an alternative
            // parser when the array length does not match the input.
            let array: [T; N] = vec
                .try_into()
                .map_err(|_| ErrMode::Backtrack(ContextError::new()))?;

            Ok(array)
        })
        .parse_next(input)
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use rstest::rstest;

    use winnow::PResult;

    use crate::extraction::{extract, Extract, HexadecimalString};

    #[rstest]
    #[case(b"[true true  false]", [true, true, false])]
    #[case(b"[1 0 -42]", [1.0, 0.0, -42.0])]
    #[case(b"[1 0]", [1, 0])]
    #[case(b"[  1   0  \r\n  ]", [1, 0])]
    #[case(b"[<00><FF>]", [HexadecimalString(vec![0x00]), HexadecimalString(vec![0xff])])]
    fn array<T>(#[case] input: &[u8], #[case] expected: T)
    where
        T: for<'a> Extract<'a> + Debug + PartialEq,
    {
        let res = extract(&mut input.as_ref()).unwrap();
        assert_eq!(expected, res);
    }

    #[rstest]
    // Too few elements.
    #[case(b"[1]")]
    // Too many elements.
    #[case(b"[1 2 3]")]
    fn array_wrong_length(#[case] input: &[u8]) {
        let res: PResult<[i32; 2]> = extract(&mut input.as_ref());
        assert!(res.is_err());
    }
}
