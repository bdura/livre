use itertools::Itertools;
use crate::parsers::{extract, Extract, Reference};
use nom::{bytes::complete::take, multi::count, IResult};

use crate::objects::Stream;
use serde::Deserialize;

use super::super::super::TrailerDict;

use super::RefLocation;

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Copy)]
#[serde(from = "(usize, usize)")]
struct SubSection {
    /// ID of the first object
    start: usize,
    /// Number of objects in the section
    n: usize,
}


impl From<(usize, usize)> for SubSection {
    fn from((start, n): (usize, usize)) -> Self {
        Self { start, n }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Copy)]
#[serde(from = "[u8;3]")]
struct FieldSize {
    f1: u8,
    // TODO: NonZeroU8?
    f2: u8,
    // Not actually useful?
    f3: u8,
}

impl FieldSize {
    #[cfg(test)]
    pub fn new(f1: u8, f2: u8, f3: u8) -> Self {
        Self { f1, f2, f3 }
    }

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

impl From<[u8; 3]> for FieldSize {
    fn from([f1, f2, f3]: [u8; 3]) -> Self {
        Self { f1, f2, f3 }
    }
}

#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(try_from="Vec<usize>")]
struct SubSections(pub Vec<SubSection>);

impl TryFrom<Vec<usize>> for SubSections {
    type Error = &'static str;

    fn try_from(value: Vec<usize>) -> Result<Self, Self::Error> {
        if value.len() % 2 != 0 {
            Err("too many integers")
        } else {
            let sub_sections: Vec<SubSection> = value.into_iter().tuples::<(_, _)>().map(SubSection::from).collect();
            Ok(SubSections(sub_sections))
        }
    }
}

impl From<Vec<SubSection>> for SubSections {
    fn from(value: Vec<SubSection>) -> Self {
        Self(value)
    }
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "PascalCase")]
struct XRefStreamConfig {
    /// Array containing sub-section info (id of first object, # objects)
    /// FIXME: use a conversion type to handle [n n n n n n ], which is necessarily interpreted as a sequence.
    /// FIXME: this is necessary because we use flattened objects, which rely on deserialize any.
    #[serde(default)]
    index: Option<SubSections>,
    /// byte offset of the previous section
    w: FieldSize,
    #[serde(flatten)]
    dict: TrailerDict,
}

#[derive(Debug, Clone, PartialEq)]
pub struct XRefStream {
    pub dict: TrailerDict,
    pub refs: Vec<(Reference, RefLocation)>,
}

impl<'input> Extract<'input> for XRefStream {
    fn extract(input: &'input [u8]) -> nom::IResult<&'input [u8], Self> {
        let (
            input,
            Stream {
                mut decoded,
                structured: XRefStreamConfig { index, w, dict },
            },
        ) = extract(input)?;

        let SubSections(index) = index.unwrap_or(vec![SubSection {
            start: 0,
            n: dict.size,
        }].into());

        let mut refs = Vec::new();

        for SubSection { start, n } in index {
            // TODO: remove this unwrap
            let (new_decoded, subsection) = count(|i| w.parse_ref(i), n)(&decoded).unwrap();
            decoded = new_decoded.into();

            let iter = subsection
                .into_iter()
                .enumerate()
                .filter_map(|(i, reference)| reference.map(|r| (Reference::first(start + i), r)));

            refs.extend(iter);
        }

        let xref_stream = Self { refs, dict };

        Ok((input, xref_stream))
    }
}

#[cfg(test)]
mod tests {

    use std::fmt::Debug;

    use indoc::indoc;
    use crate::parsers::TypedReference;
    use crate::objects::{Bytes, HexBytes};
    use crate::serde::extract_deserialize;
    use rstest::rstest;

    use super::*;

    #[test]
    fn object_stream() {
        let input = include_bytes!("../../../../tests/objects/xref_stream.bin");

        let (_, xref_stream) = XRefStream::extract(input).unwrap();

        println!("{:?}", xref_stream);
    }

    fn subsection(start: usize, n: usize) -> SubSection {
        SubSection { start, n }
    }

    #[rstest]
    #[case(b"1 1", subsection(1, 1))]
    #[case(b"91807 1006", subsection(91807, 1006))]
    #[case(b"[91807 1006 ]", vec![subsection(91807, 1006)])]
    #[case(b"[ 1 1 91807 1006]", vec![subsection(1, 1), subsection(91807, 1006)])]
    #[case(b"[ 1 1 0]", FieldSize::new(1, 1, 0))]
    #[case(b"<2B55bb>", HexBytes([0x2b, 0x55, 0xbb].to_vec()))]
    #[case(
        indoc!{b"
            <</Size 92813
            /Root 90794 0 R
            /Info 90792 0 R
            /ID[<2B551D2AFE52654494F9720283CFF1C4><6cdabf5b33a08c969604fab8979c5412>]
            /Prev 116
            /Type/XRef
            /W[ 1 3 0]
            /Index[ 1 1 7 1 91807 1006]
            /Length 1>>
        "},
        XRefStreamConfig{
            index: Some(vec![
                subsection(1, 1), 
                subsection(7, 1), 
                subsection(91807, 1006),
            ].into()),
            w: FieldSize { f1: 1, f2: 3, f3: 0 },
            dict: TrailerDict{ 
                size: 92813, 
                prev: Some(116), 
                root: TypedReference::new(90794, 0), 
                info: TypedReference::new(90792, 0), 
            }
        }
    )]
    fn deserialize<'de, T>(#[case] input: &'de [u8], #[case] expected: T)
    where
        T: Deserialize<'de> + Debug + PartialEq,
    {
        let (_, result) = extract_deserialize(input).map_err(|e| e.map_input(Bytes::from)).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn xref_stream_config() {
        let input = indoc! {b"
            <</Size 92813
            /Root 90794 0 R
            /Info 90792 0 R
            /ID[<2B551D2AFE52654494F9720283CFF1C4><6cdabf5b33a08c969604fab8979c5412>]
            /Prev 116
            /Type/XRef
            /W[ 1 3 0]
            /Index[ 1 1 7 1 91807 1006]
            /Length 1>>
            stream
            0
            endstream
        "};

        let (
            _,
            Stream {
                decoded,
                structured: XRefStreamConfig { index, w, dict },
            },
        ) = extract(input)
            .map_err(|e| e.map_input(Bytes::from))
            .unwrap();

        assert_eq!(decoded, b"0".into());
        assert_eq!(
            index.unwrap().0,
            vec![
                subsection(1, 1),
                subsection(7, 1),
                subsection(91807, 1006),
            ]
        );
        assert_eq!(w, FieldSize::new(1, 3, 0));
        assert_eq!(
            dict, 
            TrailerDict{ size: 92813, prev: Some(116), root: TypedReference::new(90794, 0), info: TypedReference::new(90792, 0) });
    }
}
