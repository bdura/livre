//! PDF encodings.
//!
//! Taken from [Junfeng Liu's `lopdf`][lopdf] crate.
//!
//! From the specification:
//!
//! > A font’s encoding is the association between character codes
//! > (obtained from text strings that are shown) and glyph descriptions.
//! > This subclause describes the character encoding scheme used with simple PDF fonts.
//! > Composite fonts (Type 0) use a different character mapping algorithm,
//!
//! [lopdf]: https://github.com/J-F-Liu/lopdf

use enum_dispatch::enum_dispatch;
use winnow::{
    error::{ContextError, ErrMode},
    BStr, PResult,
};

use crate::{
    extraction::{extract, Extract, Name},
    follow_refs::{Build, Builder},
};

mod mac_roman;
pub use mac_roman::MacRomanEncoding;

mod mac_expert;
pub use mac_expert::MacExpertEncoding;

mod win_ansi;
pub use win_ansi::WinAnsiEncoding;

mod standard;
pub use standard::StandardEncoding;

mod expert;
pub use expert::ExpertEncoding;

mod symbol;
pub use symbol::SymbolEncoding;

mod pdf_doc;
pub use pdf_doc::PdfDocEncoding;

#[enum_dispatch(Encoding)]
#[derive(Debug)]
pub enum BuiltInEncoding {
    MacRomanEncoding,
    MacExpertEncoding,
    WinAnsiEncoding,
    StandardEncoding,
    ExpertEncoding,
    SymbolEncoding,
    PdfDocEncoding,
}

impl Default for BuiltInEncoding {
    fn default() -> Self {
        PdfDocEncoding.into()
    }
}

impl Extract<'_> for BuiltInEncoding {
    fn extract(input: &mut &BStr) -> PResult<Self> {
        let Name(name) = extract(input)?;

        let inner = match name.as_slice() {
            b"MacRomanEncoding" => MacRomanEncoding.into(),
            b"MacExpertEncoding" => MacExpertEncoding.into(),
            b"WinAnsiEncoding" => WinAnsiEncoding.into(),
            b"StandardEncoding" => StandardEncoding.into(),
            b"ExpertEncoding" => ExpertEncoding.into(),
            b"SymbolEncoding" => SymbolEncoding.into(),
            b"PdfDocEncoding" => PdfDocEncoding.into(),
            _ => {
                return Err(ErrMode::Backtrack(ContextError::new()));
            }
        };

        Ok(inner)
    }
}

impl Build for BuiltInEncoding {
    fn build<B>(input: &mut &BStr, _: &B) -> PResult<Self>
    where
        B: Builder,
    {
        extract(input)
    }
}
