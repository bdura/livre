use std::fmt::Debug;

use paste::paste;

use winnow::{
    combinator::{alt, delimited, preceded, repeat, trace},
    BStr, ModalResult, Parser,
};

use crate::extraction::{
    extract, multicomment0, multicomment1, HexadecimalString, Id, LiteralString, MaybeArray, Name,
    Object, Rectangle,
};

use super::{Builder, BuilderParser, Built};

/// Generalisation on the [`Extract`](crate::extraction::Extract) trait, which allows the
/// extraction logic to follow references.
///
/// Although most `Extract` types trivially implement `Build`, we cannot use a blanket
/// implementation because of the [`OptRef`](crate::extraction::OptRef) type. Moreover,
/// this would disallow implementing `Build` for [`BuildFromRawDict`](super::BuildFromRawDict),
/// because the compiler would mark them as competing implementations.
///
/// ## Values written as references
///
/// Any value of a PDF may be written as a reference to an indirect object. `build` parses
/// the direct form only; [`Built`] is what follows a reference, and the containers
/// (`Option`, `Vec`, arrays, [`MaybeArray`]) build their items through it.
///
/// A type whose own value represents a reference sets [`FOLLOW`](Self::FOLLOW) to `false`
/// to be kept as written.
pub trait Build: Sized {
    /// Whether a reference found where a `Self` is expected is followed by [`Built`].
    ///
    /// `false` for the types that hold a reference themselves:
    /// [`Reference`](crate::extraction::Reference), [`OptRef`](crate::extraction::OptRef)
    /// and [`Object`].
    const FOLLOW: bool = true;

    /// Build an object that rely on a reference, which would be instantiated with the help of the
    /// supplied `builder`.
    ///
    /// The [`Build`] trait, like the [`Extract`](crate::extraction::Extract) trait, is a linear
    /// parser above all, hence we supply an `input`. References found during parsing, if any,
    /// are first parsed as such, and then instantiated by the `builder`.
    fn build<B>(input: &mut &BStr, builder: &B) -> ModalResult<Self>
    where
        B: Builder;
}

/// Implements [`Build`] by delegating to [`Extract`](crate::extraction::Extract), for types
/// that hold no reference; see [`Build`] for why this is not a blanket impl.
macro_rules! build_via_extract {
    ($($t:ty)+) => {
        $(
            impl $crate::follow_refs::Build for $t {
                fn build<B>(
                    input: &mut &::winnow::BStr,
                    _builder: &B,
                ) -> ::winnow::ModalResult<Self>
                where
                    B: $crate::follow_refs::Builder,
                {
                    $crate::extraction::extract(input)
                }
            }
        )+
    };
}

pub(crate) use build_via_extract;

build_via_extract!(
  i8 i16 i32 i64 i128 isize
  u8 u16 u32 u64 u128 usize
  f32 f64
  bool
  LiteralString HexadecimalString
  Id
  Name
  Rectangle
);

/// An `Object` holds a reference as [`Object::Indirect`], at any depth.
impl Build for Object {
    const FOLLOW: bool = false;

    fn build<B>(input: &mut &BStr, _: &B) -> ModalResult<Self>
    where
        B: Builder,
    {
        extract(input)
    }
}

impl<T> Build for Option<T>
where
    T: Build,
{
    fn build<B>(input: &mut &BStr, builder: &B) -> ModalResult<Self>
    where
        B: Builder,
    {
        alt((
            builder.as_parser().map(|Built(value)| Some(value)),
            b"null".map(|_| None),
        ))
        .parse_next(input)
    }
}

impl<T> Build for MaybeArray<T>
where
    T: Build,
{
    fn build<B>(input: &mut &BStr, builder: &B) -> ModalResult<Self>
    where
        B: Builder,
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

impl<T> Build for Vec<T>
where
    T: Build,
{
    fn build<B>(input: &mut &BStr, builder: &B) -> ModalResult<Self>
    where
        B: Builder,
    {
        trace(
            "livre-vec",
            delimited(
                b'[',
                repeat(
                    0..,
                    preceded(multicomment0, builder.as_parser().map(|Built(item)| item)),
                ),
                (multicomment0, b']'),
            ),
        )
        .parse_next(input)
    }
}

impl<T, const N: usize> Build for [T; N]
where
    T: Build + Debug,
{
    fn build<B>(input: &mut &BStr, builder: &B) -> ModalResult<Self>
    where
        B: Builder,
    {
        trace(
            "livre-array",
            delimited(
                b'[',
                repeat(
                    N,
                    preceded(multicomment0, builder.as_parser().map(|Built(value)| value)),
                ),
                (multicomment0, b']'),
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
            impl<$first, $($ty),+> Build for ($first, $($ty),+)
            where
                $first: Build,
                $( $ty: Build),+
            {
                fn build<B>(input: &mut &BStr, builder: &B) -> ModalResult<Self>
                where
                    B: Builder,
                {
                    trace(concat!("livre-{}-tuple", $len), move |i: &mut &BStr| {
                        let [<$first:lower>] = $first::build(i, builder)?;
                        $(
                            multicomment1(i)?;
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use rstest::rstest;

    use crate::extraction::{OptRef, Reference, ReferenceId};

    use super::*;

    /// Builds a `T` from `input` through a builder that knows objects 9 (`42`) and 10 (`43`).
    fn build<T: Build>(input: &[u8]) -> ModalResult<T> {
        let builder: HashMap<ReferenceId, &BStr> = [
            ((9, 0).into(), b"9 0 obj\n42\nendobj".as_slice().into()),
            ((10, 0).into(), b"10 0 obj\n43\nendobj".as_slice().into()),
        ]
        .into_iter()
        .collect();

        T::build(&mut input.into(), &builder)
    }

    fn reference<T>(object: usize) -> Reference<T> {
        (object, 0).into()
    }

    #[rstest]
    #[case::option(b"9 0 R", Some(42))]
    #[case::null(b"null", None)]
    fn option_follows_a_reference_to_its_item(#[case] input: &[u8], #[case] expected: Option<u32>) {
        assert_eq!(build::<Option<u32>>(input).unwrap(), expected);
    }

    #[test]
    fn containers_follow_references_to_their_items() {
        assert_eq!(build::<Vec<u32>>(b"[9 0 R 1 10 0 R]").unwrap(), [42, 1, 43]);
        assert_eq!(build::<[u32; 2]>(b"[9 0 R 10 0 R]").unwrap(), [42, 43]);
        assert_eq!(build::<MaybeArray<u32>>(b"9 0 R").unwrap().0, [42]);
    }

    #[rstest]
    #[case::reference(b"9 0 R", Some(reference(9)))]
    #[case::null(b"null", None)]
    fn option_keeps_a_reference_item(
        #[case] input: &[u8],
        #[case] expected: Option<Reference<u32>>,
    ) {
        assert_eq!(build::<Option<Reference<u32>>>(input).unwrap(), expected);
    }

    #[test]
    fn containers_keep_reference_items() {
        assert_eq!(
            build::<Vec<Reference<u32>>>(b"[9 0 R 10 0 R]").unwrap(),
            [reference(9), reference(10)]
        );
        assert_eq!(
            build::<[Reference<u32>; 2]>(b"[9 0 R 10 0 R]").unwrap(),
            [reference(9), reference(10)]
        );
        assert_eq!(
            build::<MaybeArray<Reference<u32>>>(b"[9 0 R 10 0 R]")
                .unwrap()
                .0,
            [reference(9), reference(10)]
        );
    }

    /// A lone reference is read as the single element, never as an indirect array: telling
    /// the two apart would take a look at the target.
    #[test]
    fn maybe_array_reads_a_lone_reference_as_its_single_element() {
        assert_eq!(
            build::<MaybeArray<Reference<u32>>>(b"9 0 R").unwrap().0,
            [reference(9)]
        );
    }

    #[rstest]
    #[case::reference(b"9 0 R", OptRef::Ref(reference(9)))]
    #[case::direct(b"7", OptRef::Direct(7))]
    fn option_keeps_an_optref_item_as_written(#[case] input: &[u8], #[case] expected: OptRef<u32>) {
        assert_eq!(build::<Option<OptRef<u32>>>(input).unwrap(), Some(expected));
    }

    #[test]
    fn vec_keeps_indirect_objects() {
        assert_eq!(
            build::<Vec<Object>>(b"[9 0 R 1]").unwrap(),
            [Object::Indirect(reference(9)), Object::Integer(1)]
        );
    }
}
