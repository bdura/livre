use crate::{
    extraction::{Name, Reference},
    follow_refs::BuildFromRawDict,
};
use widths::Widths;

use super::descriptor::FontDescriptor;

mod widths;

#[derive(Debug, PartialEq, Clone, BuildFromRawDict)]
pub struct SimpleFont {
    pub base_font: Name,
    #[livre(flatten)]
    pub widths: Widths,
    /// A font descriptor describing the font’s metrics other than its glyph widths
    pub font_descriptor: FontDescriptor,
    pub encoding: Option<Name>,
    pub to_unicode: Option<Reference<()>>,
}

impl SimpleFont {
    fn width(&self, code: u8) -> f32 {
        self.widths
            .width(code as usize)
            .map(f32::from)
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
