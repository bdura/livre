mod width;
use serde::Deserialize;
use width::Widths;
pub use width::WidthsTransient;

use crate::{objects::Reference, parsers::TypedReference, structure::Build};

use super::FontDescriptor;

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
