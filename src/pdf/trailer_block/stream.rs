use std::num::NonZeroU8;

use winnow::{
    combinator::{repeat, trace},
    error::ContextError,
    token::take,
    BStr, PResult, Parser,
};

use crate::{
    extraction::{extract, Extract, FromRawDict, Indirect, ReferenceId, Stream},
    pdf::Trailer,
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
    fn extract(input: &mut &BStr) -> PResult<Self> {
        let (start, n) = extract(input)?;
        Ok(Self { start, n })
    }
}

/// The field size is declared by the `/W` key in the cross-reference stream dictionary.
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
    fn extract(input: &mut &BStr) -> PResult<Self> {
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
        res += (digit as usize) * 8usize.pow(i as u32);
    }
    res
}

/// Different possible types of xref entry.
///
/// This is non-exhaustive. From the specification:
///
/// > Any other value shall be interpreted as a reference to the null object, thus permitting
/// > new entry types to be defined in the future.
#[non_exhaustive]
enum EntryType {
    /// Type `0` entries define the linked list of free (unused) objects.
    ///
    /// Livre simply discards those, since browsing a document's history is not one of Livre's
    /// goal.
    Type0,
    /// Type `1` entries define objects that are in use but uncompressed, i.e. stored as
    /// free-standing indirect objct in the PDF body.
    Type1 {
        /// Number of bytes handling the byte offset of the indirect object.
        byte_offset_len: NonZeroU8,
        /// Number of bytes occupied by the generation number.
        ///
        /// Livre does not keep track of the generation number for the cross-reference table...
        /// Since Livre is only intrested in the layout of the *current document*, we can safely
        /// disregard the generation number and focus on the ID.
        _generation_number_len: u8,
    },
    /// Type `2` entries define objects that are in use and compressed, i.e. stored within
    /// an object stream.
    Type2 {
        /// Number of bytes detailing the ID of the cross-reference stream that owns the indirect
        /// object.
        ///
        /// Note that this introduces an additional indirection for indirect object:
        ///
        /// 1. Get to the relevant ref location
        /// 2. Get the ID of the stream
        /// 3. Get to the relevant ref location
        /// 4. Parse the stream, extract the relevant part
        stream_id_len: NonZeroU8,
        index_len: u8,
    },
    /// The `Unknown` variant is used to handle additional types "gracefully".
    Unknown,
}

impl Parser<&BStr, Option<RefLocation>, ContextError> for EntryType {
    fn parse_next(&mut self, input: &mut &BStr) -> PResult<Option<RefLocation>> {
        match self {
            Self::Type0 => Ok(None),
            Self::Type1 {
                byte_offset_len,
                _generation_number_len: _,
            } => {
                let byte_offset = take(byte_offset_len.get())
                    .map(parse_num)
                    .parse_next(input)?;

                Ok(Some(RefLocation::Plain(byte_offset)))
            }
            Self::Type2 {
                stream_id_len,
                index_len,
            } => {
                let stream_id = take(stream_id_len.get()).map(parse_num).parse_next(input)?;
                let index = take(*index_len).map(parse_num).parse_next(input)?;

                Ok(Some(RefLocation::Compressed { stream_id, index }))
            }
            Self::Unknown => Ok(None),
        }
    }
}

impl FieldSize {
    /// Extract the reference entry type.
    fn parse_ref_type(&self, input: &mut &BStr) -> PResult<EntryType> {
        if self.f1 == 0 {
            Ok(EntryType::Type0)
        } else {
            let num = take(self.f1).parse_next(input)?;
            debug_assert_eq!(num.len(), 1, "f1 should only ever span one byte.");
            let entry_type = match num[0] {
                0 => EntryType::Type0,
                1 => EntryType::Type1 {
                    byte_offset_len: self.f2,
                    _generation_number_len: self.f3,
                },
                2 => EntryType::Type2 {
                    stream_id_len: self.f2,
                    index_len: self.f3,
                },
                _ => EntryType::Unknown,
            };

            Ok(entry_type)
        }
    }
}

impl Parser<&BStr, Option<RefLocation>, ContextError> for FieldSize {
    fn parse_next(&mut self, input: &mut &BStr) -> PResult<Option<RefLocation>> {
        let mut ref_type = self.parse_ref_type(input)?;
        ref_type.parse_next(input)
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
    fn extract(input: &mut &BStr) -> PResult<Self> {
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
            let subsection: Vec<Option<RefLocation>> = repeat(n, w).parse_next(input)?;

            let iter = subsection
                .into_iter()
                .enumerate()
                .filter_map(|(i, loc)| loc.map(|offset| (ReferenceId::first(start + i), offset)));

            xrefs.extend(iter);
        }

        Ok(Self(XRefTrailerBlock { xrefs, trailer }))
    }
}

pub fn block(input: &mut &BStr) -> PResult<XRefTrailerBlock> {
    trace("livre-xref-stream", move |i: &mut &BStr| {
        let Indirect {
            inner: XRefStreamBlock(block),
            ..
        } = extract(i)?;

        Ok(block)
    })
    .parse_next(input)
}
