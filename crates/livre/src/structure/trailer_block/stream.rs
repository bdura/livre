use std::num::NonZeroU8;

use winnow::{
    combinator::{repeat, trace},
    error::{ContextError, ErrMode},
    token::take,
    BStr, ModalResult, Parser,
};

use crate::{
    extraction::{extract, Extract, FromRawDict, Indirect, ReferenceId, Stream},
    structure::Trailer,
};

use super::{RefLocation, XRefTrailerBlock};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct SubSection {
    /// ID of the first object
    start: usize,
    /// Number of objects in the section
    n: usize,
}

impl Extract<'_> for SubSection {
    fn extract(input: &mut &BStr) -> ModalResult<Self> {
        let (start, n) = extract(input)?;
        Ok(Self { start, n })
    }
}

/// The field size is declared by the `W` key in the cross-reference stream dictionary.
///
/// It describes how the references are described within the stream (more precisely, how many bytes
/// are used to represent the entries). For instance, `[1 2 1]` means that the fields are one byte,
/// two bytes, and one byte, respectively.
///
/// Since `FieldSize` effectively governs the extraction logic, it implements [Parser] and
/// produces.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct FieldSize {
    f1: u8,
    f2: NonZeroU8,
    f3: u8,
}

impl Extract<'_> for FieldSize {
    fn extract(input: &mut &BStr) -> ModalResult<Self> {
        let [f1, f2, f3] = extract(input)?;
        Ok(Self {
            f1,
            f2: NonZeroU8::new(f2).expect("guaranteed by the specification"),
            f3,
        })
    }
}

fn parse_num(num: &[u8]) -> usize {
    let mut res = 0;
    for (i, &digit) in num.iter().rev().enumerate() {
        res += (digit as usize) * 256usize.pow(i as u32);
    }
    res
}

/// Reads one cross-reference stream row: three big-endian fields of `f1`, `f2` and `f3` bytes
/// (ISO 32000-2, §7.5.8.3).
///
/// The whole row is consumed whatever its type, and its location is returned only for in-use
/// entries:
///
/// - type `0`, free objects, yields `None`: Livre does not browse a document's history;
/// - type `1`, uncompressed objects, yields [`RefLocation::Plain`] at the byte offset in field 2.
///   The generation number in field 3 is ignored, since Livre only follows the current layout;
/// - type `2`, objects in an object stream, yields [`RefLocation::Compressed`] with the stream's
///   object number in field 2 and the index within it in field 3;
/// - any other type is a reference to the null object and yields `None`.
impl Parser<&BStr, Option<RefLocation>, ContextError> for FieldSize {
    fn parse_next(&mut self, input: &mut &BStr) -> Result<Option<RefLocation>, ContextError> {
        // An absent type field means type 1.
        let entry_type = if self.f1 == 0 {
            1
        } else {
            take(self.f1).map(parse_num).parse_next(input)?
        };
        let field2 = take(self.f2.get()).map(parse_num).parse_next(input)?;
        let field3 = take(self.f3).map(parse_num).parse_next(input)?;

        Ok(match entry_type {
            1 => Some(RefLocation::Plain(field2)),
            2 => Some(RefLocation::Compressed {
                stream_id: field2,
                index: field3,
            }),
            _ => None,
        })
    }
}

#[derive(Debug, PartialEq, Clone, FromRawDict)]
struct XRefStreamConfig {
    /// Array containing sub-section info (id of first object, # objects)
    index: Option<Vec<SubSection>>,
    /// byte offset of the previous section
    w: FieldSize,
    #[livre(flatten)]
    trailer: Trailer,
}

#[derive(Debug)]
struct XRefStreamBlock(pub XRefTrailerBlock);

impl Extract<'_> for XRefStreamBlock {
    fn extract(input: &mut &BStr) -> ModalResult<Self> {
        let Stream {
            content,
            structured: XRefStreamConfig { index, w, trailer },
        } = extract(input)?;

        let index = index.unwrap_or(vec![SubSection {
            start: 0,
            n: trailer.size,
        }]);

        let input = &mut content.as_slice().as_ref();

        let mut xrefs = Vec::new();

        for SubSection { start, n } in index {
            let subsection: Vec<Option<RefLocation>> =
                repeat(n, w).parse_next(input).map_err(ErrMode::Backtrack)?;

            let iter = subsection
                .into_iter()
                .enumerate()
                .filter_map(|(i, loc)| loc.map(|offset| (ReferenceId::first(start + i), offset)));

            xrefs.extend(iter);
        }

        Ok(Self(XRefTrailerBlock { xrefs, trailer }))
    }
}

pub fn block(input: &mut &BStr) -> ModalResult<XRefTrailerBlock> {
    trace("livre-xref-stream", move |i: &mut &BStr| {
        let Indirect {
            inner: XRefStreamBlock(block),
            id: _,
        } = extract(i)?;

        Ok(block)
    })
    .parse_next(input)
}

#[cfg(test)]
mod tests {

    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(&[0x00, 0x0e, 0x8a, 0x00], None)]
    #[case(&[0x00, 0x11, 0x00, 0x00], None)]
    #[case(&[0x01, 0x0e, 0x8a, 0x00], Some(RefLocation::Plain(3722)))]
    #[case(&[0x02, 0x00, 0x02, 0x00], Some(RefLocation::Compressed { stream_id: 2, index: 0}))]
    #[case(&[0x02, 0x00, 0x02, 0x01], Some(RefLocation::Compressed { stream_id: 2, index: 1}))]
    #[case(&[0x02, 0x00, 0x02, 0x02], Some(RefLocation::Compressed { stream_id: 2, index: 2}))]
    #[case(&[0x02, 0x00, 0x02, 0x03], Some(RefLocation::Compressed { stream_id: 2, index: 3}))]
    #[case(&[0x02, 0x00, 0x02, 0x04], Some(RefLocation::Compressed { stream_id: 2, index: 4}))]
    #[case(&[0x02, 0x00, 0x02, 0x05], Some(RefLocation::Compressed { stream_id: 2, index: 5}))]
    #[case(&[0x02, 0x00, 0x02, 0x06], Some(RefLocation::Compressed { stream_id: 2, index: 6}))]
    #[case(&[0x02, 0x00, 0x02, 0x07], Some(RefLocation::Compressed { stream_id: 2, index: 7}))]
    #[case(&[0x01, 0x13, 0x23, 0x00], Some(RefLocation::Plain(4899)))]
    fn extraction(#[case] input: &[u8], #[case] expected: Option<RefLocation>) {
        let mut w = FieldSize {
            f1: 1,
            f2: NonZeroU8::new(2).unwrap(),
            f3: 1,
        };
        let input = &mut input.as_ref();
        let result = w.parse_next(input).unwrap();
        assert_eq!(expected, result);
        assert!(input.is_empty(), "the whole row is consumed");
    }

    /// The first four rows of `tests/resources/text-xref-stream.pdf`: a free entry, then plain
    /// and compressed ones.
    #[test]
    fn rows_are_read_back_to_back() {
        let w = FieldSize {
            f1: 1,
            f2: NonZeroU8::new(2).unwrap(),
            f3: 1,
        };
        let input = &mut [
            0x00, 0x00, 0x00, 0x00, //
            0x01, 0x00, 0x0f, 0x00, //
            0x02, 0x00, 0x01, 0x00, //
            0x02, 0x00, 0x01, 0x01,
        ]
        .as_slice()
        .as_ref();

        let rows: Vec<Option<RefLocation>> = repeat(4, w).parse_next(input).unwrap();

        assert_eq!(
            rows,
            vec![
                None,
                Some(RefLocation::Plain(15)),
                Some(RefLocation::Compressed {
                    stream_id: 1,
                    index: 0
                }),
                Some(RefLocation::Compressed {
                    stream_id: 1,
                    index: 1
                }),
            ]
        );
        assert!(input.is_empty());
    }

    #[rstest]
    // No third field: rows are three bytes, the index of compressed objects is 0.
    #[case(1, 0, &[0x01, 0x00, 0x0e, 0x00, 0x00, 0x0f], vec![Some(RefLocation::Plain(14)), None])]
    #[case(1, 0, &[0x02, 0x00, 0x05, 0x01, 0x00, 0x10], vec![Some(RefLocation::Compressed { stream_id: 5, index: 0 }), Some(RefLocation::Plain(16))])]
    // No type field: every row is type 1.
    #[case(0, 1, &[0x0e, 0x8a, 0x00], vec![Some(RefLocation::Plain(3722))])]
    fn missing_fields(
        #[case] f1: u8,
        #[case] f3: u8,
        #[case] input: &[u8],
        #[case] expected: Vec<Option<RefLocation>>,
    ) {
        let w = FieldSize {
            f1,
            f2: NonZeroU8::new(2).unwrap(),
            f3,
        };
        let input = &mut input.as_ref();

        let rows: Vec<Option<RefLocation>> = repeat(expected.len(), w).parse_next(input).unwrap();

        assert_eq!(rows, expected);
        assert!(input.is_empty());
    }
}
