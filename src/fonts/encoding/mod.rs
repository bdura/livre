//! PDF encodings.

use enum_dispatch::enum_dispatch;

mod builtins;
pub use builtins::{
    BuiltInEncoding, ExpertEncoding, MacExpertEncoding, MacRomanEncoding, PdfDocEncoding,
    StandardEncoding, SymbolEncoding, WinAnsiEncoding,
};

/// A font's encoding is the association between character codes and glyph description.
#[enum_dispatch]
pub trait Encoding {
    /// Convert a code into a character.
    fn to_char(&self, code: u8) -> u16;
}
