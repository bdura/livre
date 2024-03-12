use livre_extraction::{extract, Extract, FromDictRef, Reference};
use livre_utilities::take_whitespace1;
use nom::{bytes::complete::take, multi::count, sequence::separated_pair, IResult};

use livre_objects::Stream;

use crate::TrailerDict;

use super::RefLocation;

#[derive(Debug)]
struct SubSection {
    /// ID of the first object
    start: usize,
    /// Number of objects in the section
    n: usize,
}

impl Extract<'_> for SubSection {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        let (input, (start, n)) =
            separated_pair(usize::extract, take_whitespace1, usize::extract)(input)?;
        Ok((input, Self { start, n }))
    }
}

#[derive(Debug)]
struct FieldSize {
    f1: u8,
    // TODO: NonZeroU8?
    f2: u8,
    // Not actually useful?
    f3: u8,
}

impl FieldSize {
    fn parse_ref_type<'i>(&self, input: &'i [u8]) -> IResult<&'i [u8], u8> {
        if self.f1 == 0 {
            Ok((input, 1))
        } else {
            let (input, num) = take(self.f1)(input)?;
            assert_eq!(num.len(), 1);
            Ok((input, num[0]))
        }
    }

    fn parse_offset<'i>(&self, input: &'i [u8]) -> IResult<&'i [u8], usize> {
        let (input, num) = take(self.f2)(input)?;
        let mut res = 0;

        for (i, &digit) in num.iter().rev().enumerate() {
            res += (digit as usize) * 16usize.pow(i as u32);
        }

        Ok((input, res))
    }

    fn parse_ref<'i>(&self, input: &'i [u8]) -> IResult<&'i [u8], Option<RefLocation>> {
        let (input, ref_type) = self.parse_ref_type(input)?;
        let (input, offset) = self.parse_offset(input)?;
        let (input, _) = take(self.f3)(input)?;

        let compressed = match ref_type {
            0 => {
                return Ok((input, None));
            }
            1 => false,
            2 => true,
            _ => unreachable!(),
        };

        let reference = RefLocation::from_offset_and_flag(offset, compressed);

        Ok((input, Some(reference)))
    }
}

impl Extract<'_> for FieldSize {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        let (input, [f1, f2, f3]) = extract::<[u8; 3]>(input)?;
        Ok((input, Self { f1, f2, f3 }))
    }
}

#[derive(Debug, FromDictRef)]
struct XRefStreamConfig {
    size: usize,
    /// Array containing sub-section info (id of first object, # objects)
    index: Option<Vec<SubSection>>,
    /// byte offset of the previous section
    prev: Option<usize>,
    w: FieldSize,
    root: Reference,
    info: Reference,
}

#[derive(Debug, Clone, PartialEq)]
pub struct XRefStream {
    pub dict: TrailerDict,
    pub refs: Vec<(Reference, RefLocation)>,
}

impl<'input> Extract<'input> for XRefStream {
    fn extract(input: &'input [u8]) -> nom::IResult<&'input [u8], Self> {
        let (input, stream) = Stream::<'input, XRefStreamConfig>::extract(input)?;

        // Remove this unwrap
        let mut decoded = &stream.decode().unwrap()[..];

        let XRefStreamConfig {
            size,
            index,
            prev,
            w,
            root,
            info,
        } = stream.structured;

        let index = index.unwrap_or(vec![SubSection { start: 0, n: size }]);

        let mut refs = Vec::new();

        for SubSection { start, n } in index {
            // TODO: remove this unwrap
            let (new_decoded, subsection) = count(|i| w.parse_ref(i), n)(decoded).unwrap();
            decoded = new_decoded;

            let iter = subsection
                .into_iter()
                .enumerate()
                .filter_map(|(i, reference)| reference.map(|r| (Reference::first(start + i), r)));

            refs.extend(iter);
        }

        let dict = TrailerDict {
            size,
            prev,
            root,
            info,
        };
        let xref_stream = Self { refs, dict };

        Ok((input, xref_stream))
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn object_stream() {
        let input = include_bytes!("../../../../tests/objects/xref_stream.bin");

        let (_, xref_stream) = XRefStream::extract(input).unwrap();

        println!("{:?}", xref_stream);
    }
}
