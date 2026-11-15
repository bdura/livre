//! CMaps: mappings from character codes to text or CIDs (ISO 32000-2, 9.7.5 and 9.10.3).
//!
//! A [`CMap`] is generic over what it maps codes to. A [`ToUnicode`] CMap maps them to
//! [`UnicodeText`]. It is read from the decoded bytes of its stream with [`CMap::parse`],
//! which never fails.

mod parse;
mod unicode;

pub(crate) use parse::{Destination, Operand};
pub use unicode::{ToUnicode, UnicodeText};

use super::Code;

/// A range of byte sequences the CMap defines codes over (ISO 32000-2, 9.7.6.2).
///
/// `low` and `high` are the values of codes of `bytes` bytes, inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodespaceRange {
    bytes: u8,
    low: u32,
    high: u32,
}

impl CodespaceRange {
    /// The number of bytes of every code in the range.
    pub const fn bytes(&self) -> u8 {
        self.bytes
    }

    /// The lowest code value of the range.
    pub const fn low(&self) -> u32 {
        self.low
    }

    /// The highest code value of the range, inclusive.
    pub const fn high(&self) -> u32 {
        self.high
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
    writing_mode: WritingMode,
}

impl<D> Default for CMap<D> {
    fn default() -> Self {
        Self {
            codespace: Vec::new(),
            singles: Vec::new(),
            ranges: Vec::new(),
            writing_mode: WritingMode::default(),
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

        if range.low.bytes() != code.bytes() || code.value() > range.high {
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

    /// The writing mode, from `/WMode`.
    pub fn writing_mode(&self) -> WritingMode {
        self.writing_mode
    }
}
