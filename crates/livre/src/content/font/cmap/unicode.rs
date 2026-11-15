//! The destination of a ToUnicode CMap: text (ISO 32000-2, 9.10.3).

use std::{fmt, ops::Deref};

use super::{parse::to_hex, CMap, Destination, Operand};

/// The text a ToUnicode CMap maps a code to: one or more UTF-16 units, decoded.
///
/// Several units are a ligature (`<00660066>` is `ff`) or a surrogate pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnicodeText(Box<str>);

/// A `/ToUnicode` CMap: from a font's character codes to the text they stand for.
pub type ToUnicode = CMap<UnicodeText>;

impl UnicodeText {
    /// The text, as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Deref for UnicodeText {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for UnicodeText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Destination for UnicodeText {
    /// A hex string of UTF-16BE units. An odd number of bytes or a lone surrogate is
    /// malformed.
    fn from_operand(operand: &Operand) -> Option<Self> {
        let bytes = match operand {
            Operand::Hex(bytes) => bytes,
            // An odd-digit hex string, already warned about when read.
            Operand::Skipped => return None,
            operand => {
                tracing::warn!("ToUnicode destination {operand:?} is not a hex string: dropped");
                return None;
            }
        };

        if bytes.len() % 2 == 1 {
            tracing::warn!(
                "ToUnicode destination {} has an odd number of bytes: dropped",
                to_hex(bytes)
            );
            return None;
        }

        let units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
            .collect();

        match String::from_utf16(&units) {
            Ok(text) => Some(Self(text.into())),
            Err(_) => {
                tracing::warn!(
                    "ToUnicode destination {} has a lone surrogate: dropped",
                    to_hex(bytes)
                );
                None
            }
        }
    }

    /// Adds `offset` to the last UTF-16 unit, as `bfrange` does (ISO 32000-2, 9.10.3). A
    /// step that overflows the unit or lands on a lone surrogate has no text.
    fn stepped(&self, offset: u32) -> Option<Self> {
        if offset == 0 {
            return Some(self.clone());
        }

        let mut units: Vec<u16> = self.0.encode_utf16().collect();
        let last = units.last_mut()?;
        *last = last.checked_add(u16::try_from(offset).ok()?)?;

        String::from_utf16(&units)
            .ok()
            .map(|text| Self(text.into()))
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::*;
    use crate::content::font::Code;

    fn text(cmap: &ToUnicode, code: Code) -> Option<String> {
        cmap.get(code).map(|text| text.to_string())
    }

    fn two(value: u16) -> Code {
        Code::two_bytes(value)
    }

    #[test]
    fn stepped_and_listed_ranges() {
        let cmap = ToUnicode::parse(indoc! {b"
            2 beginbfrange
            <0000> <0001> <0020>
            <005F> <0060> [<00660066> <00660066006C>]
            endbfrange
        "});

        assert_eq!(text(&cmap, two(0x0000)).as_deref(), Some(" "));
        assert_eq!(text(&cmap, two(0x0001)).as_deref(), Some("!"));
        assert_eq!(text(&cmap, two(0x005F)).as_deref(), Some("ff"));
        assert_eq!(text(&cmap, two(0x0060)).as_deref(), Some("ffl"));
    }

    #[test]
    fn surrogate_pairs() {
        let cmap = ToUnicode::parse(indoc! {b"
            2 beginbfchar
            <3A51> <D840DC3E>
            <0001> <D840DC3E0066>
            endbfchar
        "});

        assert_eq!(text(&cmap, two(0x3A51)).as_deref(), Some("\u{2003E}"));
        assert_eq!(text(&cmap, two(0x0001)).as_deref(), Some("\u{2003E}f"));
    }

    #[test]
    #[tracing_test::traced_test]
    fn an_odd_length_destination_is_dropped() {
        let cmap = ToUnicode::parse(indoc! {b"
            2 beginbfchar
            <0001> <0066006>
            <0002> <0067>
            endbfchar
        "});

        assert_eq!(text(&cmap, two(0x0001)), None);
        assert_eq!(text(&cmap, two(0x0002)).as_deref(), Some("g"));
        assert!(logs_contain("odd number of digits"));
    }

    #[test]
    #[tracing_test::traced_test]
    fn a_lone_surrogate_is_dropped() {
        let cmap = ToUnicode::parse(indoc! {b"
            1 beginbfchar
            <0001> <D840>
            endbfchar
        "});

        assert_eq!(text(&cmap, two(0x0001)), None);
        assert!(logs_contain("lone surrogate"));
    }

    #[test]
    fn the_encodings_fixture() {
        let cmap = ToUnicode::parse(indoc! {b"
            /CIDInit /ProcSet findresource begin
            12 dict begin
            begincmap
            /CMapName /Custom def
            1 begincodespacerange
            <00> <FF>
            endcodespacerange
            1 beginbfchar
            <41> <0061>
            endbfchar
            2 beginbfrange
            <42> <43> <0062>
            <44> <45> [<0064> <00660066>]
            endbfrange
            endcmap
            CMapName currentdict /CMap defineresource pop
            end
            end
        "});

        let one = Code::one_byte;
        assert_eq!(text(&cmap, one(0x41)).as_deref(), Some("a"));
        assert_eq!(text(&cmap, one(0x42)).as_deref(), Some("b"));
        assert_eq!(text(&cmap, one(0x43)).as_deref(), Some("c"));
        assert_eq!(text(&cmap, one(0x44)).as_deref(), Some("d"));
        assert_eq!(text(&cmap, one(0x45)).as_deref(), Some("ff"));
        assert_eq!(text(&cmap, one(0x46)), None);
        assert_eq!(cmap.codespace().len(), 1);
        assert_eq!(cmap.codespace()[0].byte_count(), 1);
        assert_eq!(
            (cmap.codespace()[0].low(), cmap.codespace()[0].high()),
            (0x00, 0xFF)
        );
    }

    #[test]
    fn one_and_two_byte_codes_do_not_collide() {
        let cmap = ToUnicode::parse(indoc! {b"
            2 beginbfchar
            <41> <0041>
            <0041> <0042>
            endbfchar
        "});

        assert_eq!(text(&cmap, Code::one_byte(0x41)).as_deref(), Some("A"));
        assert_eq!(text(&cmap, two(0x41)).as_deref(), Some("B"));
    }

    #[test]
    fn an_identity_range() {
        let cmap = ToUnicode::parse(indoc! {b"
            1 beginbfrange
            <0000> <FFFF> <0000>
            endbfrange
        "});

        assert_eq!(cmap.ranges.len(), 1);
        assert_eq!(text(&cmap, two(0x4E2D)).as_deref(), Some("中"));
    }

    #[test]
    fn a_single_mapping_beats_a_range() {
        let cmap = ToUnicode::parse(indoc! {b"
            1 beginbfchar
            <42> <0078>
            endbfchar
            1 beginbfrange
            <41> <43> <0061>
            endbfrange
        "});

        assert_eq!(text(&cmap, Code::one_byte(0x41)).as_deref(), Some("a"));
        assert_eq!(text(&cmap, Code::one_byte(0x42)).as_deref(), Some("x"));
        assert_eq!(text(&cmap, Code::one_byte(0x43)).as_deref(), Some("c"));
    }
}
