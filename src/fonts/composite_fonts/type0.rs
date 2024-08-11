use std::ops::Div;

use serde::Deserialize;

use crate::{
    fonts::FontBehavior,
    objects::Object,
    parsers::{OptRef, TypedReference},
    structure::{Build, Document, ToUnicode},
    text::operators::PdfString,
};

use super::{cidfont::CIDFontType, CIDFontTypeTransient};

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Type0Transient {
    pub descendant_fonts: OptRef<Vec<OptRef<CIDFontTypeTransient>>>,
    pub encoding: String,
    pub to_unicode: Option<TypedReference<ToUnicode>>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Type0 {
    pub descendant_font: CIDFontType,
    pub encoding: String,
    // TODO: modify object type
    pub to_unicode: Option<ToUnicode>,
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

impl Type0 {
    fn convert(&self, codepoint: u16) -> Option<Vec<char>> {
        if let Some(ToUnicode(btree)) = &self.to_unicode {
            let codes = btree.get(&codepoint)?.to_vec();
            let t = String::from_utf16_lossy(&codes);
            Some(t.chars().collect())
        } else {
            None
        }
    }

    fn width(&self, codepoint: impl Into<usize>) -> f32 {
        self.descendant_font.width(codepoint.into()) as f32 / 1000.0
    }
}

impl FontBehavior for Type0 {
    fn name(&self) -> &str {
        &self.descendant_font.base_font
    }

    fn process(&self, input: PdfString) -> Vec<(char, f32, bool)> {
        match input {
            PdfString::Utf8(input) => input
                .iter()
                .copied()
                .map(|b| (char::from(b), self.width(b), b == b' '))
                .collect(),
            PdfString::Utf16(input) => {
                let mut result = Vec::with_capacity(input.len());

                for codepoint in input {
                    let chars = self
                        .convert(codepoint)
                        .unwrap_or_else(|| vec![char::from(codepoint as u8)]);

                    let width = self.width(codepoint) / (chars.len() as f32);

                    for char in chars {
                        result.push((char, width, char == ' '));
                    }
                }

                result
            }
        }
    }

    fn ascent(&self) -> f32 {
        self.descendant_font.font_descriptor.ascent / 1000.0
    }

    fn descent(&self) -> f32 {
        self.descendant_font.font_descriptor.descent / 1000.0
    }
}
