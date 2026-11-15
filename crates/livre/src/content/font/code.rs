//! Character codes: the units a font splits a shown string into (ISO 32000-2, 9.4.3).

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
    // Field order matters: the derived `Ord` compares `bytes` first.
    bytes: u8,
    value: u32,
}

impl Code {
    /// The code read from a single byte, as simple fonts do.
    pub const fn one_byte(value: u8) -> Self {
        Self {
            value: value as u32,
            bytes: 1,
        }
    }

    /// The code read from two big-endian bytes, as `Identity-H` composite fonts do.
    pub const fn two_bytes(value: u16) -> Self {
        Self {
            value: value as u32,
            bytes: 2,
        }
    }

    /// The code read from `bytes` big-endian bytes, as a CMap's codespace defines it.
    ///
    /// `bytes` is expected in `1..=4`, the lengths a codespace range allows.
    pub(crate) const fn new(value: u32, bytes: u8) -> Self {
        Self { value, bytes }
    }

    /// The code's numeric value.
    pub const fn value(&self) -> u32 {
        self.value
    }

    /// The number of bytes the code was read from.
    pub const fn bytes(&self) -> u8 {
        self.bytes
    }

    /// Whether `Tw` applies after this code: only for the one-byte code 32.
    pub const fn word_spacing_applies(&self) -> bool {
        self.bytes == 1 && self.value == 32
    }
}

/// The codes of a shown string, as returned by [`Font::codes`].
#[derive(Debug, Clone)]
pub struct Codes<'s> {
    bytes: &'s [u8],
    two_bytes: bool,
}

impl Iterator for Codes<'_> {
    type Item = Code;

    fn next(&mut self) -> Option<Code> {
        if self.two_bytes {
            match *self.bytes {
                [high, low, ref rest @ ..] => {
                    self.bytes = rest;
                    Some(Code::two_bytes(u16::from_be_bytes([high, low])))
                }
                [byte] => {
                    self.bytes = &[];
                    tracing::warn!("Odd trailing byte {byte:#04x} in a two-byte string: dropped");
                    None
                }
                [] => None,
            }
        } else {
            let (&byte, rest) = self.bytes.split_first()?;
            self.bytes = rest;
            Some(Code::one_byte(byte))
        }
    }
}

impl Font {
    /// Split a shown string into the font's character codes.
    ///
    /// Simple and `Type3` fonts read one code per byte. Composite fonts read two bytes per
    /// code, the `Identity-H` layout whatever their `/Encoding`; an odd trailing byte is
    /// dropped with a warning.
    pub fn codes<'s>(&self, string: &'s PDFString) -> Codes<'s> {
        Codes {
            bytes: &string.0,
            two_bytes: matches!(self, Font::Composite(_)),
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
}
