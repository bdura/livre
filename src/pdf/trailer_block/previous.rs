use winnow::PResult;

use crate::extraction::{FromRawDict, RawDict};

/// Byte offset in the document of the previous xref/trailer compound (if any).
///
/// Using a dedicated type that implements [`FromRawDict`] lets us handle the `Prev` key
/// separately.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Previous {
    Linked(usize),
    Final,
}

impl FromRawDict<'_> for Previous {
    fn from_raw_dict(dict: &mut RawDict) -> PResult<Self> {
        let result = dict.pop_and_extract::<usize>(&"Prev".into());

        if let Some(res) = result {
            res.map(Self::Linked)
        } else {
            Ok(Self::Final)
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::extraction::extract;

    use super::*;

    use rstest::rstest;

    #[rstest]
    #[case(b"<</Any (type)/Of/dict>>", Previous::Final)]
    #[case(b"<</Prev 10>>", Previous::Linked(10))]
    fn extraction(#[case] input: &[u8], #[case] expected: Previous) {
        let result = extract(&mut input.as_ref()).unwrap();
        assert_eq!(expected, result)
    }
}
