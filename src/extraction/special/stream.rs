use winnow::{
    ascii::{line_ending, multispace0},
    combinator::trace,
    error::{ContextError, ErrMode},
    token::take,
    BStr, PResult, Parser,
};

use crate::{
    extraction::{extract, Extract, FromRawDict},
    filtering::{Filter, Filtering},
};

use super::{MaybeArray, Nil, RawDict};

#[derive(Debug, PartialEq, Eq, FromRawDict)]
struct StreamDict<T> {
    // FIXME: the length of a PDF stream is in fact an OptRef... This allows processors to write
    // the stream without knowing its size in advance.
    // Be that as it may, this is a failure case for the time being.
    length: usize,
    #[livre(from = MaybeArray<Filter>, default)]
    filter: Vec<Filter>,
    #[livre(flatten)]
    structured: T,
}

/// A PDF object that stores arbitrary content, as well as some (optional) structured data.
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
                length,
                filter,
                structured,
            } = StreamDict::from_raw_dict(&mut dict)?;

            (multispace0, b"stream", line_ending).parse_next(i)?;

            let content = take(length).parse_next(i)?;

            (multispace0, b"endstream").parse_next(i)?;

            let content = filter
                .decode(content)
                .map_err(|_| ErrMode::Cut(ContextError::new()))?;

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

#[cfg(test)]
mod tests {

    use std::fmt::Debug;

    use indoc::indoc;
    use rstest::rstest;

    use crate::extraction::extract;

    use super::*;

    #[rstest]
    #[case(b"<</Length 2/SomeOtherKey/Test>>", StreamDict{length: 2, filter: vec![], structured: Nil})]
    #[case(b"<</Length 42>>", StreamDict{length: 42, filter: vec![], structured: Nil})]
    #[case(b"<<  /SomeRandomKey (some text...)/Length 42>>", StreamDict{length: 42, filter: vec![], structured: Nil})]
    fn stream_dict(#[case] input: &[u8], #[case] expected: StreamDict<Nil>) {
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
