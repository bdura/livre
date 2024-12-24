use livre_derive::FromRawDict;
use winnow::{
    combinator::{repeat, trace},
    error::ContextError,
    token::take,
    BStr, PResult, Parser,
};

use crate::{
    extraction::{extract, Extract, Indirect, ReferenceId, Stream},
    pdf::Trailer,
};

use super::RefLocation;

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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct FieldSize {
    f1: u8,
    // TODO: NonZeroU8?
    f2: u8,
    // Not actually useful?
    f3: u8,
}

impl Extract<'_> for FieldSize {
    fn extract(input: &mut &BStr) -> PResult<Self> {
        let [f1, f2, f3] = extract(input)?;
        Ok(Self { f1, f2, f3 })
    }
}

impl FieldSize {
    fn parse_ref_type(&self, input: &mut &BStr) -> PResult<u8> {
        if self.f1 == 0 {
            Ok(1)
        } else {
            let num = take(self.f1).parse_next(input)?;
            debug_assert_eq!(num.len(), 1);
            Ok(num[0])
        }
    }

    fn parse_offset(&self, input: &mut &BStr) -> PResult<usize> {
        let num = take(self.f2).parse_next(input)?;
        let mut res = 0;

        for (i, &digit) in num.iter().rev().enumerate() {
            res += (digit as usize) * 16usize.pow(i as u32);
        }

        Ok(res)
    }

    fn parse_ref(&self, input: &mut &BStr) -> PResult<Option<RefLocation>> {
        let ref_type = self.parse_ref_type(input)?;
        let offset = self.parse_offset(input)?;
        take(self.f3).parse_next(input)?;

        let compressed = match ref_type {
            0 => {
                return Ok(None);
            }
            1 => false,
            2 => true,
            _ => {
                println!("found {} in {:?}", ref_type, &input[..input.len().min(100)]);

                return Ok(None);
            }
        };

        let reference = RefLocation::from_offset_and_flag(offset, compressed);

        Ok(Some(reference))
    }
}

impl Parser<&BStr, Option<RefLocation>, ContextError> for FieldSize {
    fn parse_next(&mut self, input: &mut &BStr) -> PResult<Option<RefLocation>, ContextError> {
        self.parse_ref(input)
    }
}

#[derive(Debug, PartialEq, Clone, FromRawDict)]
struct XRefStreamConfig {
    /// Array containing sub-section info (id of first object, # objects)
    index: Option<Vec<SubSection>>,
    /// byte offset of the previous section
    w: FieldSize,
    #[livre(flatten)]
    dict: Trailer,
}

#[derive(Debug, Clone, PartialEq)]
struct XRefStream {
    pub dict: Trailer,
    pub refs: Vec<(ReferenceId, RefLocation)>,
}

impl Extract<'_> for XRefStream {
    fn extract(input: &mut &BStr) -> PResult<Self> {
        let Stream {
            content,
            structured: XRefStreamConfig { index, w, dict },
        } = extract(input)?;

        let index = index.unwrap_or(vec![SubSection {
            start: 0,
            n: dict.size,
        }]);

        let input = &mut content.as_slice().as_ref();

        let mut refs = Vec::new();

        for SubSection { start, n } in index {
            let subsection: Vec<Option<RefLocation>> = repeat(n, w).parse_next(input)?;

            let iter = subsection
                .into_iter()
                .enumerate()
                .filter_map(|(i, loc)| loc.map(|offset| (ReferenceId::first(start + i), offset)));

            refs.extend(iter);
        }

        Ok(Self { refs, dict })
    }
}

pub fn xref(input: &mut &BStr) -> PResult<(Trailer, Vec<(ReferenceId, RefLocation)>)> {
    trace("livre-xref-stream", move |i: &mut &BStr| {
        let Indirect { inner, .. } = extract(i)?;
        let XRefStream { dict, refs } = inner;
        Ok((dict, refs))
    })
    .parse_next(input)
}
