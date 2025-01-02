use paste::paste;
use winnow::{BStr, PResult};

use crate::extraction::{
    extract, Extract, HexadecimalString, Id, LiteralString, MaybeArray, Name, Object, Rectangle,
};

use super::Builder;

/// Generalisation on the [`Extract`](crate::extraction::Extract) trait, which allows the
/// extraction logic to follow references.
///
/// Although most `Extract` types trivially implement `Build`, we cannot use a blanket
/// implementation because of the [`OptRef`](crate::extraction::OptRef) type.
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

macro_rules! impl_container {
    ($($t:ty)+) => {
        paste!{
            $(
                impl<'de, T> Build<'de> for $t<T>
                where
                    T: Extract<'de>,
                {
                    fn build<B>(input: &mut &'de BStr, _builder: &B) -> PResult<Self>
                    where
                        B: Builder<'de>,
                    {
                        extract(input)
                    }
                }
            )+

        }
    };
}

impl_container!(
  Vec
  MaybeArray
  Option
);

impl<'de, T, const N: usize> Build<'de> for [T; N]
where
    T: Extract<'de>,
{
    fn build<B>(input: &mut &'de BStr, _builder: &B) -> PResult<Self>
    where
        B: Builder<'de>,
    {
        extract(input)
    }
}

macro_rules! impl_tuple {
    ($len:literal: $first:ident, $($ty:ident),+) => {
        impl<'de, $first, $($ty),+> Build<'de> for ($first, $($ty),+)
        where
            $first: Extract<'de>,
            $( $ty: Extract<'de>),+
        {
            fn build<B>(input: &mut &'de BStr, _builder: &B) -> PResult<Self>
            where
                B: Builder<'de>,
            {
                extract(input)
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
