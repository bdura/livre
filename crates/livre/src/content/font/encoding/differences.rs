//! The `/Encoding` entry of a simple font: a predefined name, or an encoding dictionary with a
//! `/BaseEncoding` and `/Differences` (ISO 32000-2, 9.6.5).

use winnow::{
    ascii::multispace0,
    combinator::{alt, delimited, opt, preceded, repeat, trace},
    BStr, ModalResult, Parser,
};

use super::GlyphName;
use crate::{
    extraction::{extract, Extract, Name, Reference},
    follow_refs::{Build, BuildFromRawDict, Builder},
};

/// The `/Encoding` entry of a simple font: a predefined name, or a dictionary.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum EncodingSpec {
    /// A predefined encoding, such as `/WinAnsiEncoding`.
    Named(Name),
    /// An encoding dictionary.
    Dictionary(EncodingDict),
}

/// An encoding dictionary (ISO 32000-2, 9.6.5, Table 112).
#[derive(Debug, Clone, PartialEq, BuildFromRawDict)]
pub(crate) struct EncodingDict {
    /// The predefined encoding the differences apply to, if the dictionary names one.
    pub base_encoding: Option<Name>,
    #[livre(default)]
    pub differences: Differences,
}

/// `/Differences`: groups of a code followed by the names of the codes from it.
///
/// Every code is in `0..=255`: a group starting outside that range is dropped, and the names
/// of a group that would run past 255 are cut there, with a warning in both cases.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Differences(Vec<(u8, Vec<GlyphName>)>);

impl Differences {
    /// Every code the differences name, with its glyph name, in array order.
    pub fn iter(&self) -> impl Iterator<Item = (u8, &GlyphName)> {
        self.0.iter().flat_map(|(first, names)| {
            // Group lengths were cut at construction so that codes stay within 0..=255.
            (*first..=u8::MAX).zip(names)
        })
    }
}

/// An element of a `/Differences` array.
enum Item {
    Code(i64),
    Name(Name),
}

impl From<Vec<Item>> for Differences {
    fn from(items: Vec<Item>) -> Self {
        let mut groups: Vec<(i64, Vec<GlyphName>)> = Vec::new();
        for item in items {
            match (item, groups.last_mut()) {
                (Item::Code(code), _) => groups.push((code, Vec::new())),
                (Item::Name(name), Some((_, names))) => names.push(name.into()),
                (Item::Name(name), None) => {
                    tracing::warn!("`/Differences` name {name:?} before any code: ignored");
                }
            }
        }

        let groups = groups
            .into_iter()
            .filter_map(|(code, mut names)| {
                let Ok(first) = u8::try_from(code) else {
                    tracing::warn!("`/Differences` code {code} outside 0..=255: group ignored");
                    return None;
                };
                let room = 256 - usize::from(first);
                if names.len() > room {
                    tracing::warn!(
                        "`/Differences` group at {first} names {} glyphs: cut at code 255",
                        names.len()
                    );
                    names.truncate(room);
                }
                Some((first, names))
            })
            .collect();

        Self(groups)
    }
}

impl Extract<'_> for Differences {
    fn extract(input: &mut &BStr) -> ModalResult<Self> {
        trace(
            "livre-differences",
            delimited(
                b'[',
                repeat(
                    0..,
                    preceded(
                        multispace0,
                        alt((extract.map(Item::Code), extract.map(Item::Name))),
                    ),
                ),
                (multispace0, b']'),
            ),
        )
        .map(|items: Vec<Item>| Self::from(items))
        .parse_next(input)
    }
}

/// Reads the array inline, or through a reference.
impl Build for Differences {
    fn build<B>(input: &mut &BStr, builder: &B) -> ModalResult<Self>
    where
        B: Builder,
    {
        match opt(Reference::<Self>::extract).parse_next(input)? {
            Some(reference) => builder.build_reference(reference),
            None => Self::extract(input),
        }
    }
}

/// A name, else a dictionary; either may be behind a reference.
impl Build for EncodingSpec {
    fn build<B>(input: &mut &BStr, builder: &B) -> ModalResult<Self>
    where
        B: Builder,
    {
        if let Some(reference) = opt(Reference::<Self>::extract).parse_next(input)? {
            return builder.build_reference(reference);
        }
        trace(
            "livre-encoding-spec",
            alt((extract.map(Self::Named), |i: &mut &BStr| {
                EncodingDict::build(i, builder).map(Self::Dictionary)
            })),
        )
        .parse_next(input)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::extraction::ReferenceId;

    fn names(names: &[&'static str]) -> Vec<GlyphName> {
        names.iter().copied().map(GlyphName::from).collect()
    }

    #[test]
    fn groups_of_a_code_and_names() {
        let Differences(groups) = extract(
            &mut b"[39 /quotesingle 96 /grave 128 /Adieresis /Aring]"
                .as_slice()
                .as_ref(),
        )
        .unwrap();

        assert_eq!(
            groups,
            vec![
                (39, names(&["quotesingle"])),
                (96, names(&["grave"])),
                (128, names(&["Adieresis", "Aring"])),
            ]
        );
    }

    #[test]
    #[tracing_test::traced_test]
    fn a_code_out_of_range_drops_its_group() {
        let Differences(groups) = extract(&mut b"[300 /x]".as_slice().as_ref()).unwrap();

        assert!(groups.is_empty());
        assert!(logs_contain("`/Differences` code 300 outside 0..=255"));
    }

    #[test]
    fn iter_numbers_the_codes() {
        let differences: Differences = extract(
            &mut b"[128/Adieresis/Aring 255 /ydieresis /extra]"
                .as_slice()
                .as_ref(),
        )
        .unwrap();

        let codes: Vec<(u8, &str)> = differences
            .iter()
            .map(|(code, name)| (code, name.as_str()))
            .collect();
        assert_eq!(
            codes,
            vec![(128, "Adieresis"), (129, "Aring"), (255, "ydieresis")]
        );
    }

    #[test]
    fn spec_from_a_name() {
        let spec: EncodingSpec =
            Build::build(&mut b"/WinAnsiEncoding".as_slice().as_ref(), &()).unwrap();
        assert_eq!(spec, EncodingSpec::Named("WinAnsiEncoding".into()));
    }

    #[test]
    fn spec_from_an_inline_dictionary() {
        let spec: EncodingSpec = Build::build(
            &mut b"<</Type/Encoding/BaseEncoding/MacRomanEncoding/Differences[39/quotesingle]>>"
                .as_slice()
                .as_ref(),
            &(),
        )
        .unwrap();

        let EncodingSpec::Dictionary(EncodingDict {
            base_encoding,
            differences: Differences(groups),
        }) = spec
        else {
            panic!("not a dictionary: {spec:?}");
        };
        assert_eq!(base_encoding, Some("MacRomanEncoding".into()));
        assert_eq!(groups, vec![(39, names(&["quotesingle"]))]);
    }

    #[test]
    fn spec_through_a_reference() {
        let objects: HashMap<ReferenceId, &BStr> = [(
            (7, 0).into(),
            b"7 0 obj\n<</Differences[65/alpha]>>\nendobj"
                .as_slice()
                .as_ref(),
        )]
        .into();

        let spec: EncodingSpec = Build::build(&mut b"7 0 R".as_slice().as_ref(), &objects).unwrap();

        let EncodingSpec::Dictionary(EncodingDict {
            base_encoding: None,
            differences: Differences(groups),
        }) = spec
        else {
            panic!("not a dictionary without base: {spec:?}");
        };
        assert_eq!(groups, vec![(65, names(&["alpha"]))]);
    }
}
