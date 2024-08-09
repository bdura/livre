use serde::Deserialize;

use crate::{
    objects::Object,
    parsers::{OptRef, TypedReference},
    structure::{Build, Document},
};

use super::{cidfont::CIDFontType, CIDFontTypeTransient};

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Type0Transient {
    pub descendant_fonts: OptRef<Vec<OptRef<CIDFontTypeTransient>>>,
    pub encoding: String,
    pub to_unicode: Option<TypedReference<Object>>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Type0 {
    pub descendant_font: CIDFontType,
    pub encoding: String,
    // TODO: modify object type
    pub to_unicode: Option<Object>,
}

impl Build for Type0Transient {
    type Output = Type0;

    fn build(self, doc: &Document) -> Self::Output {
        let Self {
            descendant_fonts,
            encoding,
            to_unicode,
        } = self;

        let descendant_font = descendant_fonts
            .build(doc)
            .into_iter()
            .map(|i| i.build(doc).build(doc))
            .next()
            .expect("DescendantFonts is a one-element array");

        let to_unicode = to_unicode.map(|e| doc.parse_referenced(e));

        Type0 {
            descendant_font,
            encoding,
            to_unicode,
        }
    }
}
