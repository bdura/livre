use winnow::{
    ascii::{line_ending, multispace0},
    error::{ContextError, ErrMode},
    token::take,
    BStr, PResult, Parser,
};

use crate::{
    extraction::{extract, FromRawDict},
    filtering::{Filter, Filtering},
    Extract,
};

use super::MaybeArray;

#[derive(Debug, PartialEq, Eq, FromRawDict)]
struct StreamDict<T> {
    length: usize,
    #[livre(from = MaybeArray<Filter>, default)]
    filter: Vec<Filter>,
    #[livre(flatten)]
    structured: T,
}

#[derive(Debug)]
pub struct Stream<T> {
    pub structured: T,
    pub content: Vec<u8>,
}

impl<'de, T> Extract<'de> for Stream<T>
where
    T: FromRawDict<'de>,
{
    fn extract(input: &mut &'de BStr) -> PResult<Self> {
        let StreamDict {
            length,
            filter,
            structured,
        } = extract(input)?;

        (b" stream", line_ending).parse_next(input)?;

        let content = take(length).parse_next(input)?;

        (multispace0, b"endstream").parse_next(input)?;

        let content = filter
            .decode(content)
            .map_err(|_| ErrMode::Cut(ContextError::new()))?;

        Ok(Self {
            structured,
            content,
        })
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::extraction::extract;

    use super::*;

    #[derive(Debug, PartialEq)]
    struct NoOp;

    impl FromRawDict<'_> for NoOp {
        fn from_raw_dict(_: &mut crate::extraction::pdf::RawDict<'_>) -> PResult<Self> {
            Ok(NoOp)
        }
    }

    #[rstest()]
    #[case(b"<</Length 2/SomeOtherKey/Test>>", StreamDict{length: 2, filter: vec![], structured: NoOp})]
    #[case(b"<</Length 42>>", StreamDict{length: 42, filter: vec![], structured: NoOp})]
    #[case(b"<<  /SomeRandomKey (some text...)/Length 42>>", StreamDict{length: 42, filter: vec![], structured: NoOp})]
    fn stream_dict(#[case] input: &[u8], #[case] expected: StreamDict<NoOp>) {
        let result = extract(&mut input.as_ref()).unwrap();
        assert_eq!(expected, result);
    }
}
