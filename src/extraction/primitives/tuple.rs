use crate::extraction::Extract;

use winnow::ascii::multispace1;
use winnow::combinator::trace;
use winnow::{BStr, PResult, Parser};

use paste::paste;

macro_rules! impl_tuple {
    ($len:literal: $first:ident, $($ty:ident),+) => {
        paste! {
            impl<'de, $first, $($ty),+> Extract<'de> for ($first, $($ty),+)
            where
                $first: Extract<'de>,
                $( $ty: Extract<'de>),+
            {
                fn extract(input: &mut &'de BStr) -> PResult<Self> {
                    trace(format!("livre-{}-tuple", $len), move |i: &mut &'de BStr| {
                        let [<$first:lower>] = $first::extract(i)?;
                        $(
                            multispace1(i)?;
                            let [<$ty:lower>] = $ty::extract(i)?;
                        )*

                        let t = ([<$first:lower>], $([<$ty:lower>]),+);

                        Ok(t)
                    }).parse_next(input)
                }
            }
        }
    };
}

impl_tuple!(2: T1, T2);
impl_tuple!(3: T1, T2, T3);
impl_tuple!(4: T1, T2, T3, T4);
impl_tuple!(5: T1, T2, T3, T4, T5);
impl_tuple!(6: T1, T2, T3, T4, T5, T6);
impl_tuple!(7: T1, T2, T3, T4, T5, T6, T7);
impl_tuple!(8: T1, T2, T3, T4, T5, T6, T7, T8);
impl_tuple!(9: T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_tuple!(10: T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_tuple!(11: T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_tuple!(12: T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);

impl<'de, T> Extract<'de> for (T,)
where
    T: Extract<'de>,
{
    fn extract(input: &mut &'de BStr) -> PResult<Self> {
        trace("tuple", move |i: &mut &'de BStr| {
            let t = T::extract(i)?;
            Ok((t,))
        })
        .parse_next(input)
    }
}
#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use rstest::rstest;

    use crate::extraction::{extract, Extract};

    #[rstest]
    #[case(b"42 true", (42, true))]
    #[case(b"null true", ((), true))]
    #[case(b"1   8 true", (1, 8i32, true))]
    fn vec<T>(#[case] input: &[u8], #[case] expected: T)
    where
        T: for<'a> Extract<'a> + Debug + PartialEq,
    {
        let res = extract(&mut input.as_ref()).unwrap();
        assert_eq!(expected, res);
    }
}
