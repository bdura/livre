//! Types that describe the structure of a PDF document and their extraction strategies.
//!
//! ## How do you parse a PDF?
//!
//! From the specification:
//!
//! > PDF processors should read a PDF file from its end. The last line of the file shall
//! > contain only the end-of-file marker, `%%EOF`. The two preceding lines shall contain,
//! > one per line and in order, the keyword `startxref` and the byte offset in the decoded
//! > stream from the beginning of the PDF file to the beginning of the `xref` keyword in
//! > the last cross-reference section. The `startxref` line shall be preceded by the trailer
//! > dictionary
//!
//! Indeed, the very first step in parsing a PDF document is to obtain the cross-reference table.
//! Indeed, the PDF specification takes a "random-access" and append-only strategy to allow the
//! creation of PDFs in resource-constrained environment. Keep in mind that the technology is
//! not young, and that PDF documents may include thousands of pages with intricate graphics.
//! The random-access strategy allows PDF creators to serialise one object at a time and just
//! reference to it where needed.
//!
//! That means that most components within a PDF document are hidden behind an indirection in the
//! form of an *indirect object*. However, because knowing the indirect object's location in advance
//! would dramatically increase the memory load (since you would need to know the size of the
//! serialization before it is written to disk), the PDF specification resorts to generating
//! a cross-reference table at the end of the file to map from a reference ID to a byte location
//! within the document.
//!
//! Moreover, since the PDF specification allows modifications using an append-only strategy
//! (again for resource minimisation purposes), the full cross-reference mapping is scattered
//! accross multiple regions of the document. In practice, cross-reference tables form a linked
//! list, each new table pointing to its predecessor's byte location in the document.
//!
//! Hence, the general strategy for parsing a PDF document becomes:
//!
//! 1. Collect the full cross-reference table:
//!    1. Rush to the end of the file! You will find a [`startxref`](trailer_block::StartXRef)
//!       tag which holds the byte location of the first [cross-reference table/trailer
//!       bloc](XRefTrailerBlock).
//!    2. That table may contain a link to the previous cross-reference dictionary - if it does,
//!       follow along and continue your way up the document until you have collected the full
//!       cross-reference table.
//! 2. Iterate through the Pages dictionary.

mod content;
mod pages;
mod trailer_block;

pub use pages::Pages;
pub use trailer_block::{RefLocation, StartXRef, Trailer, XRefTrailerBlock};
