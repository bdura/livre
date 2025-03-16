//! Extraction and interation with fonts.
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
//! [Type0]: https://en.wikipedia.org/wiki/PostScript_fonts#Type_0
//! [Type1]: https://en.wikipedia.org/wiki/PostScript_fonts#Type_1
//! [MMType1]: https://en.wikipedia.org/wiki/Multiple_master_fonts
//! [Type3]: https://en.wikipedia.org/wiki/PostScript_fonts#Type_3
//! [TrueType]: https://en.wikipedia.org/wiki/TrueType
//! [CIDFont]: https://en.wikipedia.org/wiki/PostScript_fonts#CID

mod descriptor;
mod simple_fonts;
