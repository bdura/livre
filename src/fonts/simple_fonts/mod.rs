//! Behaviour for simple fonts.
//!
//! When using simple fonts, the specification states that text-showing operators
//! treat each byte within the string as a separate character code. The latter
//! then needs to be looked up in the font's encoding.
//!
//! With a simple font, each byte of the string shall be treated as a separate
//! character code. The character code shall then be looked up in the font’s
//! encoding to select the glyph, as described in 9.6.5, "Character encoding".
//!
//! Moreover, simple fonts are assigned a special behaviour for content retrieval,
//! and their encoding directly map to Unicode values.

use crate::{
    extraction::{Name, Todo},
    follow_refs::BuildFromRawDict,
};
use widths::Widths;

use super::{descriptor::FontDescriptor, encoding::Encoding};

mod widths;

#[derive(Debug, PartialEq, Clone, BuildFromRawDict)]
pub struct SimpleFont {
    pub base_font: Name,
    #[livre(flatten)]
    pub widths: Widths,
    /// A font descriptor describing the font’s metrics other than its glyph widths
    pub font_descriptor: FontDescriptor,
    /// The font's character encoding. In the absence of a `ToUnicode` key,
    /// dictates the mapping to Unicode values.
    #[livre(default)]
    pub encoding: Encoding,
    /// Optional `CMap` that maps character codes to Unicode values.
    pub to_unicode: Option<Todo>,
}

impl SimpleFont {
    pub fn width(&self, code: u8) -> f32 {
        self.widths
            .width(code as usize)
            .unwrap_or(self.font_descriptor.missing_width)
            / 1000.0
    }
}

// impl FontBehavior for SimpleFont {
//     fn ascent(&self) -> f32 {
//         self.font_descriptor.ascent / 1000.0
//     }
//
//     fn descent(&self) -> f32 {
//         self.font_descriptor.descent / 1000.0
//     }
//
//     fn process(&self, input: PdfString) -> Vec<(char, f32, bool)> {
//         Vec::<u8>::from(input)
//             .iter()
//             .copied()
//             .map(|c| (c as char, self.width(c), c == b' '))
//             .collect()
//     }
//
//     fn name(&self) -> &str {
//         &self.font_descriptor.font_name
//     }
// }
