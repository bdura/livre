//! PDF encodings.

use enum_dispatch::enum_dispatch;

mod glyphs;
pub use glyphs::Glyph;

mod builtins;
pub use builtins::{
    BuiltInEncoding, ExpertEncoding, MacExpertEncoding, MacRomanEncoding, PdfDocEncoding,
    StandardEncoding, SymbolEncoding, WinAnsiEncoding,
};

mod modified;
pub use modified::ModifiedEncoding;
use winnow::{combinator::alt, BStr, PResult, Parser};

use crate::follow_refs::{Build, Builder, BuilderParser};

pub type CharacterSet = [Option<u16>; 256];

/// A font's encoding is the association between character codes and glyph description.
#[enum_dispatch]
pub trait Decode {
    /// Convert a code into a character.
    fn decode(&self, code: u8) -> u16;
    /// Export full character set
    fn character_set(self) -> Vec<Option<u16>>;
}

/// PDF encoding.
///
/// Either a standard encoding, represented as a simple name or a more complex dictonary.
/// Default value is the `StandardEncoding`, as specified in table 112 (section 9.6.5.1)
/// in the PDF specification.
#[enum_dispatch(Decode)]
#[derive(Debug, Clone, PartialEq)]
pub enum Encoding {
    BuiltInEncoding,
    ModifiedEncoding,
}

impl Default for Encoding {
    fn default() -> Self {
        BuiltInEncoding::from(StandardEncoding).into()
    }
}

impl Build for Encoding {
    fn build<B>(input: &mut &BStr, builder: &B) -> PResult<Self>
    where
        B: Builder,
    {
        let parser = builder.as_parser();

        alt((
            parser.map(|encoding: BuiltInEncoding| encoding.into()),
            parser.map(|encoding: ModifiedEncoding| encoding.into()),
        ))
        .parse_next(input)
    }
}
