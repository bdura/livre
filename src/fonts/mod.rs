use composite_fonts::Type0;
use serde::Deserialize;

mod simple_fonts;
use simple_fonts::{SimpleFont, SimpleFontTransient};

mod composite_fonts;
pub use composite_fonts::{CIDFontTypeTransient, Type0Transient, WElement};

mod descriptors;
pub use descriptors::{FontDescriptor, FontFlags};

use crate::{parsers::Extract, serde::extract_deserialize, structure::Build};

pub trait FontBehavior {
    fn width(&self, character: usize) -> u16;
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

#[derive(Debug, Clone, PartialEq)]
pub enum Font {
    Type0(Type0),
    Simple(SimpleFont),
    // TODO: add Type3
}

impl Build for FontTransient {
    type Output = Font;

    fn build(self, doc: &crate::structure::Document) -> Self::Output {
        match self {
            Self::Type0(font) => Font::Type0(font.build(doc)),
            Self::Type1(font) => Font::Simple(font.build(doc)),
            Self::MMType1(font) => Font::Simple(font.build(doc)),
            Self::TrueType(font) => Font::Simple(font.build(doc)),
            Self::Type3 => todo!("no support for Type3 fonts yet"),
            _ => unreachable!("CIDFont are not top-level fonts"),
        }
    }
}

impl FontBehavior for Font {
    fn width(&self, character: usize) -> u16 {
        match self {
            Self::Type0(font) => font.width(character),
            Self::Simple(font) => font.width(character),
        }
    }
}
