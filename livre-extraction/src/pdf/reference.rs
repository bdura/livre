use std::{fmt, marker::PhantomData};

use nom::{bytes::complete::tag, sequence::tuple, IResult};
use serde::Deserialize;

use crate::{extract, parse, Extract};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct Reference {
    pub object: usize,
    pub generation: u16,
}

struct ReferenceVisitor;

impl<'de> serde::de::Visitor<'de> for ReferenceVisitor {
    type Value = Reference;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a PDF reference")
    }

    fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        parse(value).map_err(E::custom)
    }
}

impl<'de> Deserialize<'de> for Reference {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_bytes(ReferenceVisitor)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct TypedReference<T> {
    pub reference: Reference,
    marker: PhantomData<T>,
}

impl<'de, T> Deserialize<'de> for TypedReference<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let reference = deserializer.deserialize_bytes(ReferenceVisitor)?;
        Ok(reference.into())
    }
}

// Manual implementation is needed: otherwise it's only derived when `T` is `Clone` and `Copy`.
impl<T> Clone for TypedReference<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for TypedReference<T> {}

impl Reference {
    pub fn new(object: usize, generation: u16) -> Self {
        Self { object, generation }
    }

    pub fn first(object: usize) -> Self {
        Self {
            object,
            generation: 0,
        }
    }
}

impl<T> TypedReference<T> {
    pub fn new(object: usize, generation: u16) -> Self {
        Self {
            reference: Reference::new(object, generation),
            marker: PhantomData,
        }
    }
}

impl Extract<'_> for Reference {
    fn extract(input: &'_ [u8]) -> IResult<&'_ [u8], Self> {
        let (input, (object, _, generation, _)) =
            tuple((usize::extract, tag(" "), u16::extract, tag(" R")))(input)?;

        Ok((input, Self { object, generation }))
    }
}

impl<'input, T> Extract<'input> for TypedReference<T>
where
    T: Extract<'input>,
{
    fn extract(input: &'_ [u8]) -> IResult<&'_ [u8], Self> {
        let (input, (object, _, generation, _)) =
            tuple((extract, tag(" "), extract, tag(" R")))(input)?;

        Ok((input, Self::new(object, generation)))
    }
}

impl<T> AsRef<Reference> for TypedReference<T> {
    fn as_ref(&self) -> &Reference {
        &self.reference
    }
}

impl<T> From<TypedReference<T>> for Reference {
    fn from(value: TypedReference<T>) -> Self {
        value.reference
    }
}

impl<T> From<Reference> for TypedReference<T> {
    fn from(value: Reference) -> Self {
        let Reference { object, generation } = value;
        Self::new(object, generation)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(b"1 0 R", Reference::new(1, 0))]
    #[case(b"10 33 R", Reference::new(10, 33))]
    fn reference(#[case] input: &[u8], #[case] result: Reference) {
        let (_, reference) = Reference::extract(input).unwrap();
        assert_eq!(reference, result);
    }

    #[rstest]
    #[case(b"1 0 R", TypedReference::new(1, 0))]
    #[case(b"10 33 R", TypedReference::new(10, 33))]
    fn noop_typed_reference(#[case] input: &[u8], #[case] expected: TypedReference<()>) {
        let (_, reference) = extract(input).unwrap();
        assert_eq!(expected, reference);
    }
}
