use std::ptr;

use winnow::{
    ascii::{multispace0, multispace1},
    combinator::{delimited, separated, trace},
    BStr, PResult, Parser,
};

use crate::Extract;

impl<'de, T, const N: usize> Extract<'de> for [T; N]
where
    T: Extract<'de>,
{
    fn extract(input: &mut &'de BStr) -> PResult<Self> {
        trace(format!("livre-{N}-array"), move |i: &mut &'de BStr| {
            let mut vec: Vec<T> = delimited(
                (b'[', multispace0),
                separated(N, T::extract, multispace1),
                (multispace0, b']'),
            )
            .parse_next(i)?;

            // NOTE: the following transformation from a `Vec` (of the correct length)
            // to an array is taken from
            // <https://doc.rust-lang.org/1.80.1/src/alloc/vec/mod.rs.html#3540>
            // This allows to remove the Debug trait bound...

            // SAFETY: `.set_len(0)` is always sound.
            unsafe { vec.set_len(0) };

            // SAFETY: A `Vec`'s pointer is always aligned properly, and
            // the alignment the array needs is the same as the items.
            // We checked earlier that we have sufficient items.
            // The items will not double-drop as the `set_len`
            // tells the `Vec` not to also drop them.
            let array = unsafe { ptr::read(vec.as_ptr() as *const [T; N]) };

            Ok(array)
        })
        .parse_next(input)
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use rstest::rstest;

    use crate::{extraction::extract, Extract};

    #[rstest]
    #[case(b"[true true  false]", [true, true, false])]
    #[case(b"[1 0 -42]", [1.0, 0.0, -42.0])]
    #[case(b"[1 0]", [1, 0])]
    #[case(b"[  1   0  \r\n  ]", [1, 0])]
    fn array<T>(#[case] input: &[u8], #[case] expected: T)
    where
        T: for<'a> Extract<'a> + Debug + PartialEq,
    {
        let res = extract(&mut input.as_ref()).unwrap();
        assert_eq!(expected, res);
    }
}