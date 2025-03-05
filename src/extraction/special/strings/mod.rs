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
        write!(f, "PDFString({})", String::from_utf8_lossy(&self.0))
    }
}

impl Display for PDFString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(&self.0))
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

impl Extract<'_> for PDFString {
    fn extract(input: &mut &'_ winnow::BStr) -> winnow::PResult<Self> {
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
