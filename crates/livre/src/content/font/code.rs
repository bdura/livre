//! Character codes: the units a font splits a shown string into (ISO 32000-2, 9.4.3).

use super::cmap::CodespaceRange;
use super::Font;
use crate::extraction::PDFString;

/// A character code read from a shown string, with the number of bytes it was read from.
///
/// The length matters: word spacing applies to the one-byte code 32 only, not to a
/// multi-byte code whose value happens to be 32 (ISO 32000-2, 9.3.3).
///
/// Codes order by length, then value, so that one-byte and two-byte codes never interleave
/// in a sorted CMap table: `<41>` and `<0041>` are distinct codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Code {
    // Field order matters: the derived `Ord` compares `byte_count` first.
    byte_count: u8,
    value: u32,
}

impl Code {
    /// The code read from a single byte, as simple fonts do.
    pub const fn one_byte(value: u8) -> Self {
        Self {
            value: value as u32,
            byte_count: 1,
        }
    }

    /// The code read from two big-endian bytes, as `Identity-H` composite fonts do.
    pub const fn two_bytes(value: u16) -> Self {
        Self {
            value: value as u32,
            byte_count: 2,
        }
    }

    /// The code read from `byte_count` big-endian bytes, as a CMap's codespace defines it.
    ///
    /// `byte_count` is expected in `1..=4`, the lengths a codespace range allows.
    pub(crate) const fn new(value: u32, byte_count: u8) -> Self {
        Self { value, byte_count }
    }

    /// The code's numeric value.
    pub const fn value(&self) -> u32 {
        self.value
    }

    /// The number of bytes the code was read from.
    pub const fn byte_count(&self) -> u8 {
        self.byte_count
    }

    /// Whether `Tw` applies after this code: only for the one-byte code 32.
    pub const fn word_spacing_applies(&self) -> bool {
        self.byte_count == 1 && self.value == 32
    }
}

/// The codes of a shown string, as returned by [`Font::codes`].
#[derive(Debug, Clone)]
pub struct Codes<'s> {
    bytes: &'s [u8],
    kind: CodesKind<'s>,
}

/// How [`Codes`] splits its bytes.
#[derive(Debug, Clone, Copy)]
enum CodesKind<'s> {
    /// One code per byte, as simple and `Type3` fonts read strings.
    OneByte,
    /// Codes of the lengths a CMap's codespace ranges define (ISO 32000-2, 9.7.6.3).
    Codespace(&'s [CodespaceRange]),
}

impl<'s> Codes<'s> {
    /// Read `bytes` one code per byte.
    pub(crate) fn one_byte(bytes: &'s [u8]) -> Self {
        Self {
            bytes,
            kind: CodesKind::OneByte,
        }
    }

    /// Read `bytes` by the codespace `ranges`, with the partial-match fallback of
    /// ISO 32000-2, 9.7.6.3: a sequence no range contains still yields a code, of the
    /// length of the shortest range whose first byte matches, or of one byte.
    ///
    /// A code cut short by the end of the string is dropped with a `truncated code` warning.
    pub(crate) fn codespace(bytes: &'s [u8], ranges: &'s [CodespaceRange]) -> Self {
        Self {
            bytes,
            kind: CodesKind::Codespace(ranges),
        }
    }

    /// The next code's length under `ranges`, which may exceed the bytes left.
    fn codespace_length(&self, ranges: &[CodespaceRange]) -> usize {
        let matched = (1..=4.min(self.bytes.len())).find(|&len| {
            let prefix = &self.bytes[..len];
            ranges.iter().any(|range| range.contains(prefix))
        });
        matched.unwrap_or_else(|| {
            let first = self.bytes[0];
            ranges
                .iter()
                .filter(|range| range.first_byte_contains(first))
                .map(|range| usize::from(range.byte_count()))
                .min()
                .unwrap_or(1)
        })
    }
}

impl Iterator for Codes<'_> {
    type Item = Code;

    fn next(&mut self) -> Option<Code> {
        match self.kind {
            CodesKind::OneByte => {
                let (&byte, rest) = self.bytes.split_first()?;
                self.bytes = rest;
                Some(Code::one_byte(byte))
            }
            CodesKind::Codespace(ranges) => {
                if self.bytes.is_empty() {
                    return None;
                }
                let len = self.codespace_length(ranges);
                if self.bytes.len() < len {
                    tracing::warn!(
                        "truncated code: {:02x?} is shorter than its {len}-byte codespace range, dropped",
                        self.bytes
                    );
                    self.bytes = &[];
                    return None;
                }
                let (prefix, rest) = self.bytes.split_at(len);
                self.bytes = rest;
                let value = prefix
                    .iter()
                    .fold(0u32, |value, &byte| (value << 8) | u32::from(byte));
                // `len` is at most 4, the longest codespace range.
                Some(Code::new(value, len as u8))
            }
        }
    }
}

impl Font {
    /// Split a shown string into the font's character codes.
    ///
    /// Simple and `Type3` fonts read one code per byte. Composite fonts read codes of the
    /// lengths their encoding CMap's codespace defines, as [`Codes::codespace`] describes; a
    /// code cut short by the end of the string is dropped with a warning.
    pub fn codes<'s>(&'s self, string: &'s PDFString) -> Codes<'s> {
        match self {
            Font::Composite(font) => Codes::codespace(&string.0, font.encoding.codespace()),
            _ => Codes::one_byte(&string.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(Code::one_byte(32), true)]
    #[case(Code::two_bytes(32), false)]
    #[case(Code::one_byte(33), false)]
    fn word_spacing_applies(#[case] code: Code, #[case] expected: bool) {
        assert_eq!(code.word_spacing_applies(), expected);
    }

    const IDENTITY: &[CodespaceRange] = &[CodespaceRange::new(2, 0x0000, 0xFFFF)];
    const MIXED: &[CodespaceRange] = &[
        CodespaceRange::new(1, 0x00, 0x80),
        CodespaceRange::new(2, 0x8140, 0xFFFF),
    ];
    const TWO_BYTES_ONLY: &[CodespaceRange] = &[CodespaceRange::new(2, 0x8140, 0xFFFF)];
    const BYTE_BY_BYTE: &[CodespaceRange] = &[
        CodespaceRange::new(2, 0x8140, 0x9FFC),
        CodespaceRange::new(3, 0x81FF00, 0x81FFFF),
    ];
    const FOUR_BYTES: &[CodespaceRange] = &[CodespaceRange::new(4, 0x0000_0000, 0xFFFF_FFFF)];

    #[rstest]
    #[case::identity(IDENTITY, &[0x00, 0x41, 0x00, 0x42], &[Code::new(0x41, 2), Code::new(0x42, 2)])]
    #[case::mixed(MIXED, &[0x41, 0x81, 0x40, 0x41], &[Code::new(0x41, 1), Code::new(0x8140, 2), Code::new(0x41, 1)])]
    #[case::mixed_space(MIXED, &[0x20], &[Code::one_byte(0x20)])]
    // Second byte below the range's low: a partial match on the first byte gives two bytes.
    #[case::partial_match(MIXED, &[0x81, 0x20], &[Code::new(0x8120, 2)])]
    #[case::no_partial_match(TWO_BYTES_ONLY, &[0x41], &[Code::new(0x41, 1)])]
    // Comparing values would stop at the two-byte 0x81FF, which lies between 8140 and 9FFC.
    #[case::byte_by_byte(BYTE_BY_BYTE, &[0x81, 0xFF, 0x00], &[Code::new(0x81FF00, 3)])]
    #[case::four_bytes(FOUR_BYTES, &[0x00, 0x00, 0x00, 0x41], &[Code::new(0x41, 4)])]
    fn codespace(
        #[case] ranges: &[CodespaceRange],
        #[case] bytes: &[u8],
        #[case] expected: &[Code],
    ) {
        let codes: Vec<_> = Codes::codespace(bytes, ranges).collect();

        assert_eq!(codes, expected);
    }

    #[test]
    fn mixed_space_gets_word_spacing() {
        let code = Codes::codespace(&[0x20], MIXED).next().unwrap();

        assert!(code.word_spacing_applies());
    }

    #[test]
    fn partial_match_is_in_no_range() {
        let code = Codes::codespace(&[0x81, 0x20], MIXED).next().unwrap();

        assert_eq!(code.byte_count(), 2);
        assert!(!MIXED.iter().any(|range| range.contains(&[0x81, 0x20])));
    }
    #[test]
    #[tracing_test::traced_test]
    fn identity_odd_byte_is_truncated() {
        let codes: Vec<_> = Codes::codespace(&[0x00, 0x41, 0x00], IDENTITY).collect();

        assert_eq!(codes, [Code::new(0x41, 2)]);
        assert!(logs_contain("truncated code"));
    }

    #[test]
    #[tracing_test::traced_test]
    fn partial_match_at_the_end_is_truncated() {
        let codes: Vec<_> = Codes::codespace(&[0xFF], MIXED).collect();

        assert_eq!(codes, []);
        assert!(logs_contain("truncated code"));
    }
}
