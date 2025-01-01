use winnow::{
    combinator::{alt, trace},
    BStr, PResult, Parser,
};

use crate::extraction::{extract, Extract, Reference};

pub trait Builder<'de> {
    fn build_reference<T>(&self, reference: Reference<T>) -> PResult<T>;
}

pub trait Build<'de>: Sized {
    fn build<B: Builder<'de>>(input: &mut &'de BStr, builder: &B) -> PResult<Self>;
}

/// Implemented manually for every Extract type, except Reference & OptRef
pub trait BuildInto<'de> {
    type Target;

    fn build_into<B>(self, builder: &B) -> PResult<Self::Target>
    where
        B: Builder<'de>;
}

/// Marks a type as being "buildable" through a proxy that is itself [`Extract`].
pub trait Proxiable<'de> {
    type Proxy: Extract<'de> + BuildInto<'de, Target = Self>;
}

impl<'de, T> Proxiable<'de> for T
where
    T: Extract<'de> + BuildInto<'de, Target = T>,
{
    type Proxy = T;
}

impl<'de> BuildInto<'de> for usize {
    type Target = usize;

    fn build_into<B>(self, _builder: &B) -> PResult<Self::Target>
    where
        B: Builder<'de>,
    {
        Ok(self)
    }
}

impl<'de, T> Build<'de> for T
where
    T: Proxiable<'de>,
{
    fn build<B: Builder<'de>>(input: &mut &'de BStr, builder: &B) -> PResult<Self> {
        let proxy: T::Proxy = extract(input)?;
        proxy.build_into(builder)
    }
}

/// Helper type that abstracts away the call to OptRef.
pub struct Built<T>(T);

impl<'de, T> Build<'de> for Built<T>
where
    T: Proxiable<'de>,
{
    fn build<B: Builder<'de>>(input: &mut &'de BStr, builder: &B) -> PResult<Self> {
        trace("livre-built", move |i: &mut &'de BStr| {
            let optref: OptRef<T> = extract(i)?;
            optref.build_into(builder).map(Self)
        })
        .parse_next(input)
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum OptRef<'de, T: Proxiable<'de>> {
    Ref(Reference<T>),
    Direct(T::Proxy),
}

impl<'de, T> BuildInto<'de> for OptRef<'de, T>
where
    T: Proxiable<'de>,
{
    type Target = T;

    fn build_into<B>(self, builder: &B) -> PResult<Self::Target>
    where
        B: Builder<'de>,
    {
        match self {
            Self::Direct(inner) => inner.build_into(builder),
            Self::Ref(reference) => builder.build_reference(reference),
        }
    }
}

impl<'de, T> Extract<'de> for OptRef<'de, T>
where
    T: Proxiable<'de>,
{
    fn extract(input: &mut &'de BStr) -> PResult<Self> {
        trace(
            "livre-optref",
            alt((
                Reference::extract.map(Self::Ref),
                T::Proxy::extract.map(Self::Direct),
            )),
        )
        .parse_next(input)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::extraction::extract;

    use super::*;

    #[rstest]
    #[case(b"10", OptRef::Direct(10))]
    #[case(b"10 0 R", OptRef::Ref(Reference::<usize>::from((10, 0))))]
    fn opt_ref<'de>(#[case] input: &'de [u8], #[case] expected: OptRef<'de, usize>) {
        let result = extract(&mut input.as_ref()).unwrap();
        assert_eq!(expected, result);
    }
}
