//! PDF encodings.

use enum_dispatch::enum_dispatch;

mod glyphs;
pub use glyphs::Glyph;

mod builtins;
pub use builtins::{
    BuiltInEncoding, ExpertEncoding, MacExpertEncoding, MacRomanEncoding, PdfDocEncoding,
    StandardEncoding, SymbolEncoding, WinAnsiEncoding,
};

pub type CharacterSet = [Option<u16>; 256];

/// A font's encoding is the association between character codes and glyph description.
#[enum_dispatch]
pub trait Encoding {
    /// Convert a code into a character.
    fn to_char(&self, code: u8) -> u16;
    /// Export full character set
    fn character_set(self) -> CharacterSet;
}
