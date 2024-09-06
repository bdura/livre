use winnow::{
    ascii::{line_ending, multispace0},
    combinator::trace,
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

#[derive(Debug, PartialEq, Clone)]
pub struct Stream<T> {
    pub structured: T,
    pub content: Vec<u8>,
}

impl<'de, T> Extract<'de> for Stream<T>
where
    T: FromRawDict<'de>,
{
    fn extract(input: &mut &'de BStr) -> PResult<Self> {
        trace("livre-stream", move |i: &mut &'de BStr| {
            let StreamDict {
                length,
                filter,
                structured,
            } = extract(i)?;

            (multispace0, b"stream", line_ending).parse_next(i)?;

            let content = take(length).parse_next(i)?;

            (multispace0, b"endstream").parse_next(i)?;

            let content = filter
                .decode(content)
                .map_err(|_| ErrMode::Cut(ContextError::new()))?;

            Ok(Self {
                structured,
                content,
            })
        })
        .parse_next(input)
    }
}

#[derive(Debug, PartialEq, Clone, Copy, Eq)]
pub struct NoOp;

impl FromRawDict<'_> for NoOp {
    fn from_raw_dict(_: &mut crate::extraction::pdf::RawDict<'_>) -> PResult<Self> {
        Ok(NoOp)
    }
}

impl Extract<'_> for Stream<()> {
    fn extract(input: &mut &'_ BStr) -> PResult<Self> {
        let Stream {
            structured: NoOp,
            content,
        } = extract(input)?;

        Ok(Self {
            structured: (),
            content,
        })
    }
}

#[cfg(test)]
mod tests {

    use std::fmt::Debug;

    use indoc::indoc;
    use rstest::rstest;

    use crate::extraction::extract;

    use super::*;

    #[rstest]
    #[case(b"<</Length 2/SomeOtherKey/Test>>", StreamDict{length: 2, filter: vec![], structured: NoOp})]
    #[case(b"<</Length 42>>", StreamDict{length: 42, filter: vec![], structured: NoOp})]
    #[case(b"<<  /SomeRandomKey (some text...)/Length 42>>", StreamDict{length: 42, filter: vec![], structured: NoOp})]
    fn stream_dict(#[case] input: &[u8], #[case] expected: StreamDict<NoOp>) {
        let result = extract(&mut input.as_ref()).unwrap();
        assert_eq!(expected, result);
    }

    #[derive(Debug, PartialEq, FromRawDict)]
    struct TestStruct {
        test: bool,
    }

    #[rstest]
    #[case(
        indoc! {b"
            <</Length 10/Test true>> stream
            0123456789
            endstream
        "},
        b"0123456789",
        TestStruct {test: true},
    )]
    #[case(
        indoc! {b"
            <</Length 1/Test false /Test2 false>>stream
            0
            endstream
        "},
        b"0",
        TestStruct { test: false},
    )]
    #[case(b"<</Length 1/Test true/Test2 false>>stream\n0\nendstream", b"0", NoOp)]
    #[case(b"<</Length 1/Test false/Root true>>stream\n0\nendstream", b"0", NoOp)]
    #[case(
        b"<</Length 10/Test/Test>> stream\n0123456789\nendstream\n",
        b"0123456789",
        NoOp
    )]
    #[case(
        b"<</Length 10/Test/Test>> stream\n0123456789\nendstream\n",
        b"0123456789",
        NoOp
    )]
    fn stream<'de, T>(
        #[case] input: &'de [u8],
        #[case] expected_stream: &'static [u8],
        #[case] expected_structured: T,
    ) where
        T: FromRawDict<'de> + Debug + PartialEq,
    {
        let Stream {
            content,
            structured,
        } = extract(&mut input.as_ref()).unwrap();

        assert_eq!(expected_stream, content);
        assert_eq!(expected_structured, structured);
    }
}
