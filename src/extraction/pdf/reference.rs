use std::marker::PhantomData;

use winnow::{combinator::terminated, BStr, PResult, Parser};

use crate::{extraction::extract, Extract};

/// PDF documents resort to a "random access" strategy to limit repetition and split large objects
/// into smaller atoms.
///
/// To that end, some objects will be represented by a `Reference`, indicating the object ID as
/// well as the generation number.
///
/// I have still to understand what that means in practice... Although the definition is quite
/// simple, it looks like the generation number only takes two values: 0 or 65535.
/// Be that as it may, the `Reference` object in Livre proposes a type-safe implementation.
#[derive(Debug, PartialEq, Eq)]
pub struct Reference<T> {
    pub object: usize,
    pub generation: u16,
    _mark: PhantomData<T>,
}

impl<T> Reference<T> {
    pub fn new(object: usize, generation: u16) -> Self {
        Self {
            object,
            generation,
            _mark: PhantomData,
        }
    }
}

// We need to implement this manually because of the automatic trait bound on T.
impl<T> Clone for Reference<T> {
    fn clone(&self) -> Self {
        Self::new(self.object, self.generation)
    }
}

impl<T> Copy for Reference<T> {}

impl<T> Extract<'_> for Reference<T> {
    fn extract(input: &mut &BStr) -> PResult<Self> {
        let (object, generation) = terminated(extract, b" R").parse_next(input)?;
        Ok(Self::new(object, generation))
    }
}

impl<T> From<(usize, u16)> for Reference<T> {
    fn from((object, generation): (usize, u16)) -> Self {
        Self::new(object, generation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(b"0 0 R", (0, 0))]
    #[case(b"10 0 R", (10, 0))]
    #[case(b"10 10 R", (10, 10))]
    fn extraction(#[case] input: &[u8], #[case] expected: impl Into<Reference<()>>) {
        let result = extract(&mut input.as_ref()).unwrap();
        assert_eq!(expected.into(), result);
    }
}
