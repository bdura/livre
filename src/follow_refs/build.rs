use std::{fmt::Debug, ptr};

use paste::paste;

use winnow::{
    ascii::{multispace0, multispace1},
    combinator::{alt, delimited, preceded, repeat, trace},
    BStr, PResult, Parser,
};

use crate::extraction::{
    extract, HexadecimalString, Id, LiteralString, MaybeArray, Name, Object, Rectangle,
};

use super::{Builder, BuilderParser, Built};

/// Generalisation on the [`Extract`](crate::extraction::Extract) trait, which allows the
/// extraction logic to follow references.
///
/// Although most `Extract` types trivially implement `Build`, we cannot use a blanket
/// implementation because of the [`OptRef`](crate::extraction::OptRef) type. Moreover,
/// this would disallow implementing `Build` for [`BuildFromRawDict`](super::BuildFromRawDict),
/// because the compiler would mark them as competing implementations.
pub trait Build<'de>: Sized {
    /// Build an object that rely on a reference, which would be instantiated with the help of the
    /// supplied `builder`.
    ///
    /// The [`Build`] trait, like the [`Extract`] trait, is a linear parser above all, hence we
    /// supply an `input`. References found during parsing, if any, are first parsed as such, and
    /// then instantiated by the `builder`.
    fn build<B>(input: &mut &'de BStr, builder: &B) -> PResult<Self>
    where
        B: Builder<'de>;
}

macro_rules! impl_build_for_primitive {
    ($($t:ty)+) => {
        $(
            impl<'de> Build<'de> for $t {
                fn build<B>(input: &mut &'de BStr, _builder: &B) -> PResult<Self>
                where
                    B: Builder<'de>,
                {
                    extract(input)
                }
            }
        )+
    };
}

impl_build_for_primitive!(
  i8 i16 i32 i64 i128 isize
  u8 u16 u32 u64 u128 usize
  f32 f64
  bool
  LiteralString<'de> HexadecimalString
  Id
  Name
  Object
  Rectangle
);

impl<'de, T> Build<'de> for Option<T>
where
    T: Build<'de>,
{
    fn build<B>(input: &mut &'de BStr, builder: &B) -> PResult<Self>
    where
        B: Builder<'de>,
    {
        alt((
            builder.as_parser().map(|Built(value)| Some(value)),
            b"null".map(|_| None),
        ))
        .parse_next(input)
    }
}

impl<'de, T> Build<'de> for MaybeArray<T>
where
    T: Build<'de>,
{
    fn build<B>(input: &mut &'de BStr, builder: &B) -> PResult<Self>
    where
        B: Builder<'de>,
    {
        trace(
            "livre-vec",
            alt((
                builder.as_parser().map(|Built(value)| vec![value]),
                builder.as_parser(),
            )),
        )
        .map(Self)
        .parse_next(input)
    }
}

impl<'de, T> Build<'de> for Vec<T>
where
    T: Build<'de>,
{
    fn build<B>(input: &mut &'de BStr, builder: &B) -> PResult<Self>
    where
        B: Builder<'de>,
    {
        trace(
            "livre-vec",
            delimited(
                b'[',
                repeat(
                    0..,
                    preceded(multispace0, builder.as_parser().map(|Built(item)| item)),
                ),
                (multispace0, b']'),
            ),
        )
        .parse_next(input)
    }
}

impl<'de, T, const N: usize> Build<'de> for [T; N]
where
    T: Build<'de> + Debug,
{
    fn build<B>(input: &mut &'de BStr, builder: &B) -> PResult<Self>
    where
        B: Builder<'de>,
    {
        trace(
            concat!("livre-{N}-array"),
            delimited(
                b'[',
                repeat(
                    N,
                    preceded(multispace0, builder.as_parser().map(|Built(value)| value)),
                ),
                (multispace0, b']'),
            ),
        )
        .map(|values: Vec<T>| {
            <[T; N]>::try_from(values).expect("correct number of elements by construction")
        })
        .parse_next(input)
    }
}

macro_rules! impl_tuple {
    ($len:literal: $first:ident, $($ty:ident),+) => {
        paste!{
            impl<'de, $first, $($ty),+> Build<'de> for ($first, $($ty),+)
            where
                $first: Build<'de>,
                $( $ty: Build<'de>),+
            {
                fn build<B>(input: &mut &'de BStr, builder: &B) -> PResult<Self>
                where
                    B: Builder<'de>,
                {
                    trace(concat!("livre-{}-tuple", $len), move |i: &mut &'de BStr| {
                        let [<$first:lower>] = $first::build(i, builder)?;
                        $(
                            multispace1(i)?;
                            let [<$ty:lower>] = $ty::build(i, builder)?;
                        )*

                        let t = ([<$first:lower>], $([<$ty:lower>]),+);

                        Ok(t)
                    }).parse_next(input)
                }
            }
        }
    }
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
