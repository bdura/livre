//! CMaps: mappings from character codes to text or CIDs (ISO 32000-2, 9.7.5 and 9.10.3).
//!
//! A [`CMap`] is generic over what it maps codes to. A [`ToUnicode`] CMap maps them to
//! [`UnicodeText`]. An encoding CMap maps them to [`Cid`]s, and is wrapped in an
//! [`EncodingCMap`] that also covers the predefined identities. A CMap is read from the
//! decoded bytes of its stream with [`CMap::parse`], which never fails.

mod cid;
mod parse;
mod unicode;

pub(crate) use cid::IDENTITY_CODESPACE;
pub use cid::{Cid, EncodingCMap};
pub(crate) use parse::{Destination, Operand};
pub use unicode::{ToUnicode, UnicodeText};

use super::Code;

/// A range of byte sequences the CMap defines codes over (ISO 32000-2, 9.7.6.2).
///
/// `low` and `high` are the values of codes of `byte_count` bytes, inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodespaceRange {
    byte_count: u8,
    low: u32,
    high: u32,
}

impl CodespaceRange {
    /// The range of `byte_count`-byte codes from `low` to `high`, inclusive.
    pub(crate) const fn new(byte_count: u8, low: u32, high: u32) -> Self {
        Self {
            byte_count,
            low,
            high,
        }
    }

    /// The number of bytes of every code in the range.
    pub const fn byte_count(&self) -> u8 {
        self.byte_count
    }

    /// The lowest code value of the range.
    pub const fn low(&self) -> u32 {
        self.low
    }

    /// The highest code value of the range, inclusive.
    pub const fn high(&self) -> u32 {
        self.high
    }

    /// Whether every byte of `bytes` lies between the matching bytes of `low` and `high`.
    ///
    /// The check is byte by byte, not by value (ISO 32000-2, 9.7.6.2): `<8140> <9FFC>` does
    /// not contain `<81FF>`, whose second byte is above `FC`. `false` when the lengths differ.
    pub(crate) fn contains(&self, bytes: &[u8]) -> bool {
        bytes.len() == usize::from(self.byte_count)
            && bytes
                .iter()
                .enumerate()
                .all(|(index, &byte)| self.byte_interval(index).contains(&byte))
    }

    /// Whether `byte` lies in the range's interval of first bytes: a partial match
    /// (ISO 32000-2, 9.7.6.3).
    pub(crate) fn first_byte_contains(&self, byte: u8) -> bool {
        self.byte_interval(0).contains(&byte)
    }

    /// The interval the byte at `index`, counted from the most significant, ranges over.
    fn byte_interval(&self, index: usize) -> std::ops::RangeInclusive<u8> {
        let shift = 8 * (usize::from(self.byte_count) - 1 - index);
        let low = (self.low >> shift) as u8;
        let high = (self.high >> shift) as u8;
        low..=high
    }
}

/// The direction a CMap lays its glyphs out in, from its `/WMode` entry (ISO 32000-2, 9.7.5.3).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WritingMode {
    /// `/WMode 0`, or no `/WMode`.
    #[default]
    Horizontal,
    /// `/WMode 1`.
    Vertical,
}

/// A mapping from character codes to `D`, as a ToUnicode or an encoding CMap defines it.
///
/// Ranges are kept as ranges: an identity range over two bytes is one entry, not 65536.
#[derive(Debug, Clone, PartialEq)]
pub struct CMap<D> {
    codespace: Vec<CodespaceRange>,
    /// `bfchar` and `cidchar` entries, sorted by code, one per code.
    singles: Vec<(Code, D)>,
    /// `bfrange` and `cidrange` entries, sorted by `low`.
    ranges: Vec<Range<D>>,
    /// From `/WMode n def` in the body, or `usecmap` of `Identity-V`; `None` when neither
    /// appears, so that a stream dictionary's `/WMode` can stand in.
    writing_mode: Option<WritingMode>,
    /// Whether the body reads `/Identity-H usecmap` or `/Identity-V usecmap`: codes it does
    /// not map fall back to the identity.
    use_identity: bool,
}

impl<D> Default for CMap<D> {
    fn default() -> Self {
        Self {
            codespace: Vec::new(),
            singles: Vec::new(),
            ranges: Vec::new(),
            writing_mode: None,
            use_identity: false,
        }
    }
}

/// The codes from `low` to `high`, of the same byte length as `low`.
#[derive(Debug, Clone, PartialEq)]
struct Range<D> {
    low: Code,
    high: u32,
    destination: RangeDestination<D>,
}

#[derive(Debug, Clone, PartialEq)]
enum RangeDestination<D> {
    /// One destination for `low`, stepped forward for each following code.
    Stepped(D),
    /// One destination per code, the array form.
    Listed(Vec<D>),
}

impl<D: Destination> CMap<D> {
    /// What `code` maps to, if anything.
    ///
    /// A single mapping wins over a range covering the same code, matching how producers
    /// write exceptions to a range.
    pub fn get(&self, code: Code) -> Option<D> {
        if let Ok(index) = self.singles.binary_search_by_key(&code, |(code, _)| *code) {
            return Some(self.singles[index].1.clone());
        }

        let index = self.ranges.partition_point(|range| range.low <= code);
        let range = &self.ranges[index.checked_sub(1)?];

        if range.low.byte_count() != code.byte_count() || code.value() > range.high {
            return None;
        }

        let offset = code.value() - range.low.value();
        match &range.destination {
            RangeDestination::Stepped(destination) => destination.stepped(offset),
            RangeDestination::Listed(destinations) => {
                destinations.get(usize::try_from(offset).ok()?).cloned()
            }
        }
    }
}

impl<D> CMap<D> {
    /// The codespace ranges, as `begincodespacerange` declares them.
    pub fn codespace(&self) -> &[CodespaceRange] {
        &self.codespace
    }

    /// Whether the CMap maps no code at all.
    pub fn is_empty(&self) -> bool {
        self.singles.is_empty() && self.ranges.is_empty()
    }

    /// The writing mode, from `/WMode`; horizontal when the body declares none.
    pub fn writing_mode(&self) -> WritingMode {
        self.writing_mode.unwrap_or_default()
    }

    /// Whether the CMap inherits from `Identity-H` or `Identity-V` through `usecmap`.
    pub fn use_identity(&self) -> bool {
        self.use_identity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(&[0x81, 0x40], true)]
    #[case(&[0x9F, 0xFC], true)]
    #[case(&[0x90, 0x80], true)]
    // In by value, out by its second byte.
    #[case(&[0x81, 0xFF], false)]
    #[case(&[0x80, 0x50], false)]
    #[case(&[0x81], false)]
    fn contains(#[case] bytes: &[u8], #[case] expected: bool) {
        let range = CodespaceRange::new(2, 0x8140, 0x9FFC);

        assert_eq!(range.contains(bytes), expected);
    }
}
