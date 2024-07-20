use livre_extraction::{FromDictRef, Name, TypedReference};

use crate::descriptors::FontDescriptor;

#[derive(Debug, PartialEq, FromDictRef)]
pub struct Widths {
    /// The first character code defined in the font’s `widths` array
    pub first_char: usize,
    /// The last character code defined in the font’s `widths` array
    pub last_char: usize,
    /// An array of `last_char - first_char + 1` numbers, each element being
    /// the glyph width for the character code that equals FirstChar plus the array index.
    /// For character codes outside the range `first_char` to `last_char`, the value of
    /// `missing_width` from the FontDescriptor entry for this font shall be used. The
    /// glyph widths shall be measured in units in which 1000 units correspond to 1 unit
    /// in text space.
    pub width: Vec<f32>,
}

#[derive(Debug, PartialEq, FromDictRef)]
pub struct Type1FontDict {
    #[livre(from = Name)]
    pub base_font: String,
    #[livre(flatten)]
    pub widths: Widths,
    /// A font descriptor describing the font’s metrics other than its glyph widths
    pub font_descriptor: TypedReference<FontDescriptor>,
}
