use std::fmt::Debug;

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
    follow_refs::{Build, BuildFromRawDict, Builder, BuilderParser, Built},
};

use super::{MaybeArray, Nil, RawDict};

/// The `StreamConfig` contains everything needed to read the stream content, starting with the
/// `length` of the encoded content, and the filters that should be apply for decoding.
///
/// The full stream dictionary is represented by the [`StreamDict`] instance.
///
/// Since `StreamConfig` is needed to extract the content of a stream, Livre implements [`Parser`]
/// for it.
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

/// Represents the dictionary part of the stream. A PDF stream is made of two parts:
///
/// 1. A dictionary that contains stream-specific properties (e.g. length of the encoded content,
///    filters, etc)
/// 2. The encoded stream content itself. You'll need the former to extract it, in particular the
///    `length` field.
///
/// In Livre, `StreamDict<T>` is a generic container for the former.
#[derive(Debug, PartialEq, Eq, FromRawDict)]
struct StreamDict<T> {
    #[livre(flatten)]
    config: StreamConfig,
    #[livre(flatten)]
    structured: T,
}

impl BuildFromRawDict for StreamConfig {
    fn build_from_raw_dict<B>(dict: &mut RawDict<'_>, builder: &B) -> PResult<Self>
    where
        B: Builder,
    {
        let Built(length) = dict
            .pop_and_build(&b"Length".into(), builder)?
            .ok_or(ErrMode::Backtrack(ContextError::new()))?;

        let filter = dict
            .pop_and_build::<Built<MaybeArray<Filter>>, _>(&b"Filter".into(), builder)?
            .map(|Built(filter)| filter)
            .unwrap_or_default();

        let filter = filter.into();

        Ok(Self { length, filter })
    }
}

impl<T> BuildFromRawDict for StreamDict<T>
where
    T: BuildFromRawDict,
{
    fn build_from_raw_dict<B>(dict: &mut RawDict<'_>, builder: &B) -> PResult<Self>
    where
        B: Builder,
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
/// - store page content (any dictionary field may contain point to an indirect object)
/// - store cross-references (no references)
///
/// In Livre, PDF-specific structured properties are considered implementation details, and an
/// extracted stream only contains what you need, that is:
///
/// - the structured data, if any
/// - the actual, decoded content
#[derive(PartialEq, Clone)]
pub struct Stream<T> {
    pub structured: T,
    pub content: Vec<u8>,
}

impl<T> Debug for Stream<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let utfish = String::from_utf8_lossy(&self.content[..100.min(self.content.len())]);

        f.debug_struct("Stream")
            .field("structured", &self.structured)
            .field("content", &utfish)
            .finish()
    }
}

impl<'de, T> Extract<'de> for Stream<T>
where
    T: FromRawDict<'de>,
{
    fn extract(input: &mut &'de BStr) -> PResult<Self> {
        trace("livre-stream", move |i: &mut &'de BStr| {
            let mut dict: RawDict = extract(i)?;
            let StreamDict {
                mut config,
                structured,
            } = StreamDict::from_raw_dict(&mut dict)?;

            let content = config.parse_next(i)?;

            Ok(Self {
                structured,
                content,
            })
        })
        .parse_next(input)
    }
}

impl<T> Build for Stream<T>
where
    T: BuildFromRawDict,
{
    fn build<B>(input: &mut &BStr, builder: &B) -> PResult<Self>
    where
        B: Builder,
    {
        trace("livre-stream", move |i: &mut &BStr| {
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

impl Build for Stream<()> {
    fn build<B>(input: &mut &BStr, builder: &B) -> PResult<Self>
    where
        B: Builder,
    {
        let Stream {
            structured: Nil,
            content,
        } = builder.as_parser().parse_next(input)?;

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
