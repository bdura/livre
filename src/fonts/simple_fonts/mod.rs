mod width;
use serde::Deserialize;
pub use width::WidthsTransient;

use crate::{objects::Reference, parsers::TypedReference};

use super::FontDescriptor;

#[derive(Debug, PartialEq, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SimpleFont {
    pub base_font: String,
    #[serde(flatten)]
    pub widths: WidthsTransient,
    /// A font descriptor describing the font’s metrics other than its glyph widths
    pub font_descriptor: TypedReference<FontDescriptor>,
    pub encoding: Option<String>,
    pub to_unicode: Option<Reference>,
}
