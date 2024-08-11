use composite_fonts::Type0;
use enum_dispatch::enum_dispatch;
use serde::Deserialize;

mod simple_fonts;
use simple_fonts::{SimpleFont, SimpleFontTransient};

mod composite_fonts;
pub use composite_fonts::{CIDFontTypeTransient, Type0Transient, WElement};

mod descriptors;
pub use descriptors::{FontDescriptor, FontFlags};

use crate::{
    parsers::Extract, serde::extract_deserialize, structure::Build, text::operators::PdfString,
};

#[enum_dispatch]
pub trait FontBehavior {
    fn process(&self, input: PdfString) -> Vec<(char, f32, bool)> {
        match input {
            PdfString::Utf8(input) => input
                .iter()
                .copied()
                .map(|b| (char::from(b), 0.5, b == b' '))
                .collect(),
            PdfString::Utf16(input) => input
                .iter()
                .copied()
                .map(|b| b as u8)
                .map(|b| (char::from(b), 0.5, b == b' '))
                .collect(),
        }
    }
    fn ascent(&self) -> f32 {
        0.0
    }
    fn descent(&self) -> f32 {
        0.0
    }
    fn name(&self) -> &str {
        "undefined"
    }
}

#[derive(Debug, PartialEq, Deserialize, Clone)]
#[serde(tag = "Subtype", rename_all = "PascalCase")]
pub enum FontTransient {
    Type0(Type0Transient),
    Type1(SimpleFontTransient),
    MMType1(SimpleFontTransient),
    TrueType(SimpleFontTransient),
    // TODO: add Type3 font dict
    Type3,
    CIDFontType0(CIDFontTypeTransient),
    CIDFontType2(CIDFontTypeTransient),
}

impl Extract<'_> for FontTransient {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        extract_deserialize(input)
    }
}

#[enum_dispatch(FontBehavior)]
#[derive(Debug, Clone, PartialEq)]
pub enum Font {
    Type0,
    SimpleFont,
    // TODO: add Type3
}

impl Build for FontTransient {
    type Output = Font;

    fn build(self, doc: &crate::structure::Document) -> Self::Output {
        match self {
            Self::Type0(font) => Font::Type0(font.build(doc)),
            Self::Type1(font) => Font::SimpleFont(font.build(doc)),
            Self::MMType1(font) => Font::SimpleFont(font.build(doc)),
            Self::TrueType(font) => Font::SimpleFont(font.build(doc)),
            Self::Type3 => todo!("no support for Type3 fonts yet"),
            _ => unreachable!("CIDFont are not top-level fonts"),
        }
    }
}
