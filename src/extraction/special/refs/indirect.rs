use winnow::{
    ascii::multispace1,
    combinator::{delimited, trace},
    BStr, PResult, Parser,
};

use crate::extraction::{extract, Extract};

use super::ReferenceId;

/// The source for an indirect object, in the PDF body. It can be referenced in the PDF using a
/// [`Reference`](super::Reference).
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Indirect<T> {
    pub id: ReferenceId,
    pub inner: T,
}

impl<'de, T> From<(ReferenceId, T)> for Indirect<T>
where
    T: Extract<'de>,
{
    fn from((id, inner): (ReferenceId, T)) -> Self {
        Self { id, inner }
    }
}

/// We go the extra mile and extract the trailing `endobj` tag. This is not actually needed,
/// although it does serve as a kind of sanity check.
impl<'de, T> Extract<'de> for Indirect<T>
where
    T: Extract<'de>,
{
    fn extract(input: &mut &'de BStr) -> PResult<Self> {
        trace(
            "livre-indirect",
            (
                extract,
                delimited((b" obj", multispace1), extract, (multispace1, b"endobj")),
            )
                .map(Self::from),
        )
        .parse_next(input)
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(b"0 0 obj\n10\nendobj", Indirect{id: (0, 0).into(), inner: 10})]
    #[case(b"0 0 obj\ntrue\nendobj", Indirect{id: (0, 0).into(), inner: true})]
    #[case(b"0 0 obj\n10    true\nendobj", Indirect{id: (0, 0).into(), inner: (10i32, true)})]
    fn extraction<'de, T>(#[case] input: &'de [u8], #[case] expected: Indirect<T>)
    where
        T: Extract<'de> + Debug + PartialEq,
    {
        let result = extract(&mut input.as_ref()).unwrap();
        assert_eq!(expected, result);
    }
}
