use nom::IResult;

use super::take_whitespace;
use super::Extract;
use paste::paste;

macro_rules! impl_tuple {
    ($first:ident, $($ty:ident),+) => {
        paste! {
            impl<'input, $first, $($ty),+> Extract<'input> for ($first, $($ty),+)
            where
                $first: Extract<'input>,
                $( $ty: Extract<'input>),+
            {
                fn extract(input: &'input [u8]) -> IResult<&'input [u8], Self> {
                    let (input, [<$first:lower>]) = $first::extract(input)?;
                    $(
                        let (input, _) = take_whitespace(input)?;
                        let (input, [<$ty:lower>]) = $ty::extract(input)?;
                    )*

                    let t = ([<$first:lower>], $([<$ty:lower>]),+);

                    Ok((input, t))
                }
            }
        }
    };
}

impl_tuple!(T1, T2);
impl_tuple!(T1, T2, T3);
impl_tuple!(T1, T2, T3, T4);
impl_tuple!(T1, T2, T3, T4, T5);
impl_tuple!(T1, T2, T3, T4, T5, T6);
impl_tuple!(T1, T2, T3, T4, T5, T6, T7);
impl_tuple!(T1, T2, T3, T4, T5, T6, T7, T8);
impl_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::parsers::extraction::extract;

    #[rstest]
    #[case(b"(test) 23", ("test".to_string(), 23))]
    #[case(b"(test)23", ("test".to_string(), 23))]
    #[case(b"(haha) -13", ("haha".to_string(), -13))]
    fn tuple2(#[case] input: &[u8], #[case] expected: (String, i16)) {
        let (_, result) = extract(input).unwrap();
        assert_eq!(expected, result);
    }

    #[rstest]
    #[case(b"true 23 -2.3", (true, 23, -2.3))]
    #[case(b"false -23 0", (false, -23, 0.0))]
    fn tuple3(#[case] input: &[u8], #[case] expected: (bool, i16, f32)) {
        let (_, result) = extract(input).unwrap();
        assert_eq!(expected, result);
    }
}
