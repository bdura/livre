use winnow::{
    combinator::{separated_pair, trace},
    BStr, PResult, Parser,
};

use crate::extraction::{extract, Extract};

/// An ID that uniquely identifies an object and its version.
///
/// In practice, it looks like the [`object`](Self::object) field alone
/// should be enough for text extraction since the XRef dictionary is
/// updated such that only the latest version of a given object is referenced.
///
/// In the future we *might* want to look at the document's history,
/// hence the [`ReferenceId`] keeps the generation number.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct ReferenceId {
    pub object: usize,
    pub generation: u16,
}

impl ReferenceId {
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

impl Extract<'_> for ReferenceId {
    fn extract(input: &mut &BStr) -> PResult<Self> {
        trace(
            "livre-reference-id",
            separated_pair(extract, b' ', extract).map(ReferenceId::from),
        )
        .parse_next(input)
    }
}

impl From<(usize, u16)> for ReferenceId {
    fn from((object, generation): (usize, u16)) -> Self {
        Self::new(object, generation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(b"0 0", (0, 0))]
    #[case(b"10 0", (10, 0))]
    #[case(b"10 10", (10, 10))]
    fn reference_id(#[case] input: &[u8], #[case] expected: impl Into<ReferenceId>) {
        let result = extract(&mut input.as_ref()).unwrap();
        assert_eq!(expected.into(), result);
    }
}
