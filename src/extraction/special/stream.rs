use winnow::{
    ascii::{line_ending, multispace0},
    combinator::{delimited, trace},
    error::{ContextError, ErrMode},
    token::take,
    BStr, PResult, Parser,
};

use crate::{
    extraction::{extract, Extract, FromRawDict},
    filtering::{Filter, Filtering},
    follow_refs::{Build, BuildFromRawDict, Builder, Built},
};

use super::{MaybeArray, Nil, RawDict};

#[derive(Debug, PartialEq, Eq, FromRawDict)]
pub struct StreamConfig {
    length: usize,
    #[livre(from = MaybeArray<Filter>, default)]
    filter: Vec<Filter>,
}

impl Parser<&BStr, Vec<u8>, ContextError> for StreamConfig {
    fn parse_next(&mut self, input: &mut &BStr) -> PResult<Vec<u8>> {
        let content = trace(
            "livre-stream-content",
            delimited(
                (multispace0, b"stream", line_ending),
                take(self.length),
                (multispace0, b"endstream"),
            ),
        )
        .parse_next(input)?;

        self.filter
            .decode(content)
            .map_err(|_| ErrMode::Cut(ContextError::new()))
    }
}

#[derive(Debug, PartialEq, Eq, FromRawDict)]
struct StreamDict<T> {
    #[livre(flatten)]
    config: StreamConfig,
    #[livre(flatten)]
    structured: T,
}

impl<'de> BuildFromRawDict<'de> for StreamConfig {
    fn build_from_raw_dict<B>(dict: &mut RawDict<'de>, builder: &B) -> PResult<Self>
    where
        B: Builder<'de>,
    {
        let Built(length) = dict
            .pop_and_build(&b"Length".into(), builder)
            .ok_or(ErrMode::Backtrack(ContextError::new()))??;

        let Built(filter) = dict
            .pop_and_build(&b"Filter".into(), builder)
            .ok_or(ErrMode::Backtrack(ContextError::new()))??;

        Ok(Self { length, filter })
    }
}

impl<'de, T> BuildFromRawDict<'de> for StreamDict<T>
where
    T: BuildFromRawDict<'de>,
{
    fn build_from_raw_dict<B>(dict: &mut RawDict<'de>, builder: &B) -> PResult<Self>
    where
        B: Builder<'de>,
    {
        let config = StreamConfig::build_from_raw_dict(dict, builder)?;
        let structured = T::build_from_raw_dict(dict, builder)?;

        Ok(Self { config, structured })
    }
}

/// A PDF object that stores arbitrary content, as well as some (optional) structured data.
///
/// PDF streams can be used to:
///
/// - store page content (any dictionary data may contain indirect objects)
/// - store cross-references (no references)
#[derive(Debug, PartialEq, Clone)]
pub struct Stream<T> {
    pub structured: T,
    pub content: Vec<u8>,
}

impl<'de, T> Stream<T>
where
    T: FromRawDict<'de>,
{
    /// Extract the stream, and returns the partially consumed dictionary for later use.
    pub fn extract_with_dict(input: &mut &'de BStr) -> PResult<(Self, RawDict<'de>)> {
        trace("livre-stream-dict", move |i: &mut &'de BStr| {
            let mut dict: RawDict = extract(i)?;
            let StreamDict {
                mut config,
                structured,
            } = StreamDict::from_raw_dict(&mut dict)?;

            let content = config.parse_next(i)?;

            Ok((
                Self {
                    structured,
                    content,
                },
                dict,
            ))
        })
        .parse_next(input)
    }
}

impl<'de, T> Extract<'de> for Stream<T>
where
    T: FromRawDict<'de>,
{
    fn extract(input: &mut &'de BStr) -> PResult<Self> {
        trace("livre-stream", move |i: &mut &'de BStr| {
            let (stream, _) = Self::extract_with_dict(i)?;
            Ok(stream)
        })
        .parse_next(input)
    }
}

impl<'de, T> Build<'de> for Stream<T>
where
    T: BuildFromRawDict<'de>,
{
    fn build<B>(input: &mut &'de BStr, builder: &B) -> PResult<Self>
    where
        B: Builder<'de>,
    {
        trace("livre-stream", move |i: &mut &'de BStr| {
            let mut dict: RawDict = extract(i)?;
            let StreamDict {
                mut config,
                structured,
            } = StreamDict::build_from_raw_dict(&mut dict, builder)?;

            let content = config.parse_next(i)?;

            Ok(Self {
                structured,
                content,
            })
        })
        .parse_next(input)
    }
}

impl Extract<'_> for Stream<()> {
    fn extract(input: &mut &'_ BStr) -> PResult<Self> {
        let Stream {
            structured: Nil,
            content,
        } = extract(input)?;

        Ok(Self {
            structured: (),
            content,
        })
    }
}

impl<'de> Build<'de> for Stream<()> {
    fn build<B>(input: &mut &'de BStr, builder: &B) -> PResult<Self>
    where
        B: Builder<'de>,
    {
        trace("livre-stream", move |i: &mut &'de BStr| {
            let mut dict: RawDict = extract(i)?;
            let StreamDict {
                mut config,
                structured: Nil,
            } = StreamDict::build_from_raw_dict(&mut dict, builder)?;

            let content = config.parse_next(i)?;

            Ok(Self {
                structured: (),
                content,
            })
        })
        .parse_next(input)
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
    #[case(b"<</Length 2/SomeOtherKey/Test>>", StreamConfig{length: 2, filter: vec![]})]
    #[case(b"<</Length 42>>", StreamConfig{length: 42, filter: vec![]})]
    #[case(b"<<  /SomeRandomKey (some text...)/Length 42>>", StreamConfig{length: 42, filter: vec![]})]
    fn stream_config(#[case] input: &[u8], #[case] expected: StreamConfig) {
        let result = extract(&mut input.as_ref()).unwrap();
        assert_eq!(expected, result);
    }

    //#[rstest]
    //#[case(b"<</Length 2/SomeOtherKey/Test>>", StreamDict{length: 2, filter: vec![], structured: Nil})]
    //#[case(b"<</Length 42>>", StreamDict{length: 42, filter: vec![], structured: Nil})]
    //#[case(b"<<  /SomeRandomKey (some text...)/Length 42>>", StreamDict{length: 42, filter: vec![], structured: Nil})]
    //fn stream_dict(#[case] input: &[u8], #[case] expected: StreamDict<Nil>) {
    //    let result = extract(&mut input.as_ref()).unwrap();
    //    assert_eq!(expected, result);
    //}

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
    #[case(b"<</Length 1/Test true/Test2 false>>stream\n0\nendstream", b"0", Nil)]
    #[case(b"<</Length 1/Test false/Root true>>stream\n0\nendstream", b"0", Nil)]
    #[case(
        b"<</Length 10/Test/Test>> stream\n0123456789\nendstream\n",
        b"0123456789",
        Nil
    )]
    #[case(
        b"<</Length 10/Test/Test>> stream\n0123456789\nendstream\n",
        b"0123456789",
        Nil
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
