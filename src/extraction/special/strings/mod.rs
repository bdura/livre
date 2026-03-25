//! The PDF specs are quite convoluted when it comes to string definition.
//! In fact, even though in most cases a PDF string maps with the actual
//! text that is rendered on the page, there is **absolutely no guarantee**
//! that this is the case.
//!
//! In effect, we have to consider a PDF "string" as an array of bytes,
//! whose translation into an actual text can only be performed knowing the
//! encoding, along with the font used for rendering.
//!
//! This is at least my current understanding of how PDFs work. In any case,
//! this is the reason why "PDF strings" are actually stored as bytes within
//! Livre.

mod hex_string;
mod literal_string;

use std::fmt::{Debug, Display};

pub use hex_string::HexadecimalString;
pub use literal_string::LiteralString;
use winnow::combinator::{alt, trace};
use winnow::Parser;

use crate::extraction::Extract;

#[derive(Clone, PartialEq)]
pub struct PDFString(pub Vec<u8>);

impl Debug for PDFString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PDFString({})", self.decode())
    }
}

impl Display for PDFString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.decode())
    }
}

impl From<&str> for PDFString {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

impl From<HexadecimalString> for PDFString {
    fn from(HexadecimalString(value): HexadecimalString) -> Self {
        Self(value)
    }
}

impl From<LiteralString> for PDFString {
    fn from(LiteralString(value): LiteralString) -> Self {
        Self(value)
    }
}

impl PDFString {
    /// Decode the raw PDF string bytes into a Rust `String` using best-effort heuristics.
    ///
    /// PDF strings carry no explicit encoding metadata at the byte level. The correct
    /// decoding depends on the font's `Encoding` entry, `ToUnicode` CMap, and whether
    /// the font is a simple or composite font — none of which are available here.
    ///
    /// Until full font infrastructure is in place, we apply the following heuristics in order:
    ///
    /// 1. **UTF-16BE BOM** (`\xFE\xFF`): decoded as UTF-16BE. Surrogate pairs are handled by
    ///    `char::decode_utf16`; unpaired surrogates are replaced with U+FFFD.
    /// 2. **Otherwise**: bytes are interpreted as ISO 8859-1 (Latin-1). Every byte value maps
    ///    directly to the Unicode codepoint of the same value — always lossless.
    ///    Covers ASCII-range PDFs and WinAnsi-encoded strings.
    pub fn decode(&self) -> String {
        let bytes = &self.0;

        if bytes.starts_with(&[0xFE, 0xFF]) {
            // Skip the two-byte BOM, then group remaining bytes into u16 pairs.
            // Odd trailing bytes (malformed input) are silently dropped by chunks_exact.
            let utf16_units: Vec<u16> = bytes[2..]
                .chunks_exact(2)
                .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
                .collect();

            char::decode_utf16(utf16_units)
                .map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER))
                .collect()
        } else {
            // ISO 8859-1: the Unicode codepoint equals the byte value for all 256 values,
            // so char::from_u32 is infallible here.
            bytes
                .iter()
                .map(|&b| {
                    char::from_u32(b as u32).expect("all u8 values are valid Latin-1 codepoints")
                })
                .collect()
        }
    }
}

impl Extract<'_> for PDFString {
    fn extract(input: &mut &'_ winnow::BStr) -> winnow::ModalResult<Self> {
        trace(
            "livre-pdfstring",
            alt((
                HexadecimalString::extract.map(PDFString::from),
                LiteralString::extract.map(PDFString::from),
            )),
        )
        .parse_next(input)
    }
}
