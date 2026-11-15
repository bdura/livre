//! The `/Encoding` entry of a simple font: a predefined name, or an encoding dictionary with a
//! `/BaseEncoding` and `/Differences` (ISO 32000-2, 9.6.5).

use winnow::{
    ascii::multispace0,
    combinator::{alt, iterator, opt, preceded, trace},
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
    /// The codes the dictionary renames, empty when `/Differences` is absent.
    #[livre(default)]
    pub differences: Differences,
}

/// `/Differences`: groups of a code followed by the names of the codes from it, flattened to
/// one glyph name per code.
///
/// Every code is in `0..=255`: a group starting outside that range is dropped, and the names
/// of a group that would run past 255 are cut there, with a warning in both cases.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Differences(Vec<(u8, GlyphName)>);

impl Differences {
    /// Every code the differences name, with its glyph name, in array order.
    pub fn iter(&self) -> impl Iterator<Item = (u8, &GlyphName)> {
        self.0.iter().map(|(code, name)| (*code, name))
    }
}

/// An element of a `/Differences` array.
enum Item {
    Code(i64),
    Name(Name),
}

/// Where the next name of a `/Differences` array goes.
enum Cursor {
    /// No code yet.
    Start,
    /// The code of the next name: 256 once the group has reached code 255.
    At(u16),
    /// The current group is dropped or cut.
    Skipping,
}

/// The codes and glyph names of the elements of a `/Differences` array, warning about the
/// names it drops.
struct Codes<I> {
    items: I,
    cursor: Cursor,
}

impl<I: Iterator<Item = Item>> Iterator for Codes<I> {
    type Item = (u8, GlyphName);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match (self.items.next()?, &self.cursor) {
                (Item::Code(code), _) => {
                    self.cursor = match u8::try_from(code) {
                        Ok(code) => Cursor::At(code.into()),
                        Err(_) => {
                            tracing::warn!(
                                "`/Differences` code {code} outside 0..=255: group ignored"
                            );
                            Cursor::Skipping
                        }
                    };
                }
                (Item::Name(name), Cursor::Start) => {
                    tracing::warn!("`/Differences` name {name:?} before any code: ignored");
                }
                (Item::Name(_), Cursor::Skipping) => {}
                (Item::Name(name), &Cursor::At(code)) => match u8::try_from(code) {
                    Ok(code) => {
                        self.cursor = Cursor::At(u16::from(code) + 1);
                        return Some((code, name.into()));
                    }
                    Err(_) => {
                        tracing::warn!("`/Differences` name {name:?} past code 255: group cut");
                        self.cursor = Cursor::Skipping;
                    }
                },
            }
        }
    }
}

impl Extract<'_> for Differences {
    fn extract(input: &mut &BStr) -> ModalResult<Self> {
        trace("livre-differences", |input: &mut &BStr| {
            b'['.parse_next(input)?;
            let mut items = iterator(
                input,
                preceded(
                    multispace0,
                    alt((extract.map(Item::Code), extract.map(Item::Name))),
                ),
            );
            let differences = Self(
                Codes {
                    items: &mut items,
                    cursor: Cursor::Start,
                }
                .collect(),
            );
            items.finish()?;
            (multispace0, b']').parse_next(input)?;
            Ok(differences)
        })
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

    fn codes(codes: &[(u8, &'static str)]) -> Vec<(u8, GlyphName)> {
        codes
            .iter()
            .map(|&(code, name)| (code, GlyphName::from(name)))
            .collect()
    }

    #[test]
    fn groups_number_their_names_from_their_code() {
        let Differences(names) = extract(
            &mut b"[39 /quotesingle 96 /grave 128 /Adieresis /Aring]"
                .as_slice()
                .as_ref(),
        )
        .unwrap();

        assert_eq!(
            names,
            codes(&[
                (39, "quotesingle"),
                (96, "grave"),
                (128, "Adieresis"),
                (129, "Aring"),
            ])
        );
    }

    #[test]
    #[tracing_test::traced_test]
    fn a_name_before_any_code_is_ignored() {
        let Differences(names) = extract(&mut b"[/x 65 /A]".as_slice().as_ref()).unwrap();

        assert_eq!(names, codes(&[(65, "A")]));
        assert!(logs_contain("before any code: ignored"));
    }

    #[test]
    #[tracing_test::traced_test]
    fn a_code_out_of_range_drops_its_group() {
        let Differences(names) = extract(&mut b"[300 /x /y 65 /A]".as_slice().as_ref()).unwrap();

        assert_eq!(names, codes(&[(65, "A")]));
        assert!(logs_contain("`/Differences` code 300 outside 0..=255"));
    }

    #[test]
    #[tracing_test::traced_test]
    fn a_group_past_255_is_cut() {
        let Differences(names) = extract(
            &mut b"[128/Adieresis/Aring 255 /ydieresis /extra /more 0 /A]"
                .as_slice()
                .as_ref(),
        )
        .unwrap();

        assert_eq!(
            names,
            codes(&[
                (128, "Adieresis"),
                (129, "Aring"),
                (255, "ydieresis"),
                (0, "A")
            ])
        );
        assert!(logs_contain("past code 255: group cut"));
    }

    #[test]
    fn an_unclosed_array_fails() {
        assert!(extract::<Differences>(&mut b"[65 /A".as_slice().as_ref()).is_err());
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
            differences: Differences(names),
        }) = spec
        else {
            panic!("not a dictionary: {spec:?}");
        };
        assert_eq!(base_encoding, Some("MacRomanEncoding".into()));
        assert_eq!(names, codes(&[(39, "quotesingle")]));
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
            differences: Differences(names),
        }) = spec
        else {
            panic!("not a dictionary without base: {spec:?}");
        };
        assert_eq!(names, codes(&[(65, "alpha")]));
    }
}
