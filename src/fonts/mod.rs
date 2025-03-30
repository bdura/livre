//! Extraction and interaction with fonts.
//!
//! PDF documents support a variety of font types:
//!
//! | Type      | Subtype      |
//! | --------- | ------------ |
//! | Type 0    | [Type0]      |
//! | Type 1    | [Type1]      |
//! | Type 1    | [MMType1]    |
//! | Type 3    | [Type3]      |
//! | TrueType  | [TrueType]   |
//! | [CIDFont] | CIDFontType0 |
//! | [CIDFont] | CIDFontType2 |
//!
//! Livre follows the same hierarchy as the PDF specification when in comes to fonts.
//!
//! ## General considerations
//!
//! ### Text positioning
//!
//! Glyphs are positioned according to a cursor that is translated by each glyph's width.
//! The width may be constant, but most fonts associate a different width with each glyph.
//! In any case, the width information is stored in the from dictionary.
//!
//! Note that the font program itself also stores that information, albeit with a slightly
//! different strategy. `TrueType` fonts store widths in units of 1024 or 2048th of an Em,
//! which means that the two definition do not perfectly line up.
//! To limit the discrepancy, the `Width` array in the font's dictionary entry often uses
//! real numbers.
//!
//! ### Coordinate systems
//!
//! The PDF specification defines multiple coordinate systems:
//!
//! - The **glyph coordinate system** is the space in which a glyph is defined.
//!   When painting a glyph, its origin is placed at the origin of the text space.
//!   The units of glyph space are one-thousandth of a unit of text space, unless
//!   the font is Type 3 - in that case, an explicit `FontMatrix` is defined by
//!   the font dictionary.
//! - Text space
//! - User space
//!
//!
//! [Type0]: https://en.wikipedia.org/wiki/PostScript_fonts#Type_0
//! [Type1]: https://en.wikipedia.org/wiki/PostScript_fonts#Type_1
//! [MMType1]: https://en.wikipedia.org/wiki/Multiple_master_fonts
//! [Type3]: https://en.wikipedia.org/wiki/PostScript_fonts#Type_3
//! [TrueType]: https://en.wikipedia.org/wiki/TrueType
//! [CIDFont]: https://en.wikipedia.org/wiki/PostScript_fonts#CID

pub mod encoding;

mod descriptor;
mod simple_fonts;
