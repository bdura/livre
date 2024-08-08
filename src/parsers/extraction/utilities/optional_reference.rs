use nom::{branch::alt, combinator::map, IResult};
use serde::Deserialize;

use crate::structure::Document;

use super::super::{Extract, TypedReference};

#[derive(Debug, PartialEq, Clone, Deserialize)]
#[serde(untagged)]
pub enum OptRef<T> {
    Val(T),
    Ref(TypedReference<T>),
}

impl<'input, T> Extract<'input> for OptRef<T>
where
    T: Extract<'input>,
{
    fn extract(input: &'input [u8]) -> IResult<&'input [u8], Self> {
        alt((
            map(TypedReference::extract, Self::Ref),
            map(T::extract, Self::Val),
        ))(input)
    }
}

impl<T> OptRef<T> {
    pub fn is_ref(&self) -> bool {
        matches!(self, Self::Ref(_))
    }

    pub fn is_val(&self) -> bool {
        matches!(self, Self::Val(_))
    }
}

impl<'a, T> OptRef<T>
where
    T: Extract<'a>,
{
    pub fn get_or_instantiate(&mut self, doc: &'a Document) -> &mut T {
        match self {
            Self::Val(val) => val,
            Self::Ref(reference) => {
                let obj = doc.parse_referenced(*reference);
                *self = Self::Val(obj);

                match self {
                    Self::Val(val) => val,
                    _ => unreachable!(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(b"0 1 R", OptRef::Ref(TypedReference::new(0, 1)))]
    #[case(b"-12", OptRef::Val(-12))]
    #[case(b"10 1 R", OptRef::Ref(TypedReference::new(10, 1)))]
    fn maybe_array_i32(#[case] input: &[u8], #[case] expected: OptRef<i32>) {
        let (_, opt_ref) = OptRef::<i32>::extract(input).unwrap();
        assert_eq!(opt_ref, expected);
    }
}
