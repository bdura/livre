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
//! For now, like [`lopdf`][lopdf], Livre uses an array of 256 `Option<u16>`
//! to represent built-it encodings. Although using `[T; 256]` is probably
//! the right strategy, we may switch to using `&str` instead of `u16` -
//! which would bring some advantages, among which:
//!
//! - direct utf-8 conversion
//! - more general approach, allowing complex utf-8 strings to be used
//!   (see example 2 from section 9.10.3 in the specification, which discusses
//!   the representation of the `ff`, `fi` and `ffi` ligatures)
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

/// Built-in PDF encoding that maps single-byte character codes to Unicode values.
#[enum_dispatch(Decode)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
