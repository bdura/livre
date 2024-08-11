mod width;
use serde::Deserialize;
use width::Widths;
pub use width::WidthsTransient;

use crate::{
    objects::Reference, parsers::TypedReference, structure::Build, text::operators::PdfString,
};

use super::{FontBehavior, FontDescriptor};

#[derive(Debug, PartialEq, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SimpleFontTransient {
    pub base_font: String,
    #[serde(flatten)]
    pub widths: WidthsTransient,
    /// A font descriptor describing the font’s metrics other than its glyph widths
    pub font_descriptor: TypedReference<FontDescriptor>,
    pub encoding: Option<String>,
    pub to_unicode: Option<Reference>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct SimpleFont {
    pub base_font: String,
    pub widths: Widths,
    pub font_descriptor: FontDescriptor,
    pub encoding: Option<String>,
    // TODO: add to_unicode
    // pub to_unicode: Option<Reference>,
}

impl Build for SimpleFontTransient {
    type Output = SimpleFont;

    fn build(self, doc: &crate::structure::Document) -> Self::Output {
        let Self {
            base_font,
            widths,
            font_descriptor,
            encoding,
            ..
        } = self;

        let font_descriptor = font_descriptor.build(doc);
        let widths = widths.build(doc);

        SimpleFont {
            base_font,
            widths,
            font_descriptor,
            encoding,
        }
    }
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

impl FontBehavior for SimpleFont {
    fn ascent(&self) -> f32 {
        self.font_descriptor.ascent / 1000.0
    }

    fn descent(&self) -> f32 {
        self.font_descriptor.descent / 1000.0
    }

    fn process(&self, input: PdfString) -> Vec<(char, f32, bool)> {
        Vec::<u8>::from(input)
            .iter()
            .copied()
            .map(|c| (c as char, self.width(c), c == b' '))
            .collect()
    }

    fn name(&self) -> &str {
        &self.font_descriptor.font_name
    }
}
