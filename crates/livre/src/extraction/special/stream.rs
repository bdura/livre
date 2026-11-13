use std::fmt::Debug;

use winnow::{
    ascii::{line_ending, multispace0},
    combinator::{delimited, trace},
    error::{ContextError, ErrMode},
    token::take,
    BStr, ModalResult, Parser,
};

use crate::{
    extraction::{extract, Extract, FromRawDict},
    filtering::{Filter, Filtering, FlateParams},
    follow_refs::{Build, BuildFromRawDict, Builder, BuilderParser},
};

use super::{MaybeArray, Nil, RawDict};

/// The `StreamConfig` contains everything needed to read the stream content, starting with the
/// `length` of the encoded content, and the filters that should be apply for decoding.
///
/// The full stream dictionary is represented by the [`StreamDict`] instance.
///
/// Since `StreamConfig` is needed to extract the content of a stream, Livre implements [`Parser`]
/// for it.
#[derive(Debug, PartialEq, Eq, FromRawDict, BuildFromRawDict)]
pub struct StreamConfig {
    length: usize,
    #[livre(from = MaybeArray<Filter>, default)]
    filter: Vec<Filter>,
    /// Parallel to `filter`, `None` where an array entry is `null`. Not
    /// checked against `filter`'s length: a missing entry means no
    /// parameters, extra entries are ignored.
    #[livre(from = MaybeArray<Option<FlateParams>>, default)]
    decode_parms: Vec<Option<FlateParams>>,
}

impl Parser<&BStr, Vec<u8>, ErrMode<ContextError>> for StreamConfig {
    fn parse_next(&mut self, input: &mut &BStr) -> ModalResult<Vec<u8>> {
        let content = trace(
            "livre-stream-content",
            delimited(
                (multispace0, b"stream", line_ending),
                take(self.length),
                (multispace0, b"endstream"),
            ),
        )
        .parse_next(input)?;

        let mut decoded = content.to_vec();
        for (i, filter) in self.filter.iter().enumerate() {
            let params = self.decode_parms.get(i).copied().flatten();
            decoded = filter
                .decode(&decoded, params.as_ref())
                .map_err(|_| ErrMode::Cut(ContextError::new()))?;
        }
        Ok(decoded)
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
#[derive(Debug, PartialEq, Eq, FromRawDict, BuildFromRawDict)]
struct StreamDict<T> {
    #[livre(flatten)]
    config: StreamConfig,
    #[livre(flatten)]
    structured: T,
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
    fn extract(input: &mut &'de BStr) -> ModalResult<Self> {
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
    fn build<B>(input: &mut &BStr, builder: &B) -> ModalResult<Self>
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
    fn extract(input: &mut &'_ BStr) -> ModalResult<Self> {
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
    fn build<B>(input: &mut &BStr, builder: &B) -> ModalResult<Self>
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

    use std::{collections::HashMap, fmt::Debug, io::Write};

    use flate2::{write::ZlibEncoder, Compression};
    use indoc::indoc;
    use rstest::rstest;

    use crate::{
        extraction::{extract, ReferenceId},
        filtering::FlateDecode,
    };

    use super::*;

    fn flate_config(length: usize, decode_parms: Vec<Option<FlateParams>>) -> StreamConfig {
        StreamConfig {
            length,
            filter: vec![Filter::FlateDecode(FlateDecode)],
            decode_parms,
        }
    }

    fn params(predictor: u8, columns: usize) -> FlateParams {
        FlateParams {
            predictor,
            colors: 1,
            bits_per_component: 8,
            columns,
        }
    }

    #[rstest]
    #[case(b"<</Length 2/SomeOtherKey/Test>>", StreamConfig{length: 2, filter: vec![], decode_parms: vec![]})]
    #[case(b"<</Length 42>>", StreamConfig{length: 42, filter: vec![], decode_parms: vec![]})]
    #[case(b"<<  /SomeRandomKey (some text...)/Length 42>>", StreamConfig{length: 42, filter: vec![], decode_parms: vec![]})]
    #[case(b"<</Length 3/Filter/FlateDecode>>", flate_config(3, vec![]))]
    #[case(
        b"<</Length 3/Filter /FlateDecode /DecodeParms <</Predictor 12/Columns 4>>>>",
        flate_config(3, vec![Some(params(12, 4))]),
    )]
    #[case(
        b"<</Length 3/Filter [/FlateDecode /FlateDecode] /DecodeParms [null <</Columns 5>>]>>",
        StreamConfig {
            length: 3,
            filter: vec![Filter::FlateDecode(FlateDecode); 2],
            decode_parms: vec![None, Some(params(1, 5))],
        },
    )]
    #[case(
        b"<</Length 3/Filter/FlateDecode/DecodeParms<</EarlyChange 0/Predictor 2/Columns 3>>>>",
        flate_config(3, vec![Some(params(2, 3))]),
    )]
    fn stream_config(#[case] input: &[u8], #[case] expected: StreamConfig) {
        let result = extract(&mut input.as_ref()).unwrap();
        assert_eq!(expected, result);
    }

    /// Two 4-byte rows, PNG-Up predicted, then zlib-compressed.
    fn png_up_rows() -> (Vec<u8>, Vec<u8>) {
        let raw = vec![1, 2, 3, 4, 2, 3, 4, 5];
        let predicted = [2, 1, 2, 3, 4, 2, 1, 1, 1, 1];

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&predicted).unwrap();

        (encoder.finish().unwrap(), raw)
    }

    fn predicted_stream(decode_parms: &str) -> (Vec<u8>, Vec<u8>) {
        let (compressed, raw) = png_up_rows();

        let mut input = format!(
            "<</Length {}/Filter/FlateDecode/DecodeParms{decode_parms}>>stream\n",
            compressed.len()
        )
        .into_bytes();
        input.extend_from_slice(&compressed);
        input.extend_from_slice(b"\nendstream\n");

        (input, raw)
    }

    #[test]
    fn stream_undoes_predictor() {
        let (input, raw) = predicted_stream("<</Predictor 12/Columns 4>>");

        let Stream { content, .. }: Stream<()> = extract(&mut input.as_slice().as_ref()).unwrap();

        assert_eq!(raw, content);
    }

    #[test]
    fn stream_undoes_predictor_behind_reference() {
        let (input, raw) = predicted_stream(" 5 0 R");

        let mut builder: HashMap<ReferenceId, &BStr> = HashMap::new();
        builder.insert(
            ReferenceId::first(5),
            b"5 0 obj\n<</Predictor 12/Columns 4>>\nendobj"
                .as_slice()
                .as_ref(),
        );

        let Stream { content, .. }: Stream<()> =
            Stream::build(&mut input.as_slice().as_ref(), &builder).unwrap();

        assert_eq!(raw, content);
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
