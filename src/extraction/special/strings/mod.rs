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

pub use hex_string::HexadecimalString;
pub use literal_string::LiteralString;

// TODO: create a PDFString enum/newtype?
