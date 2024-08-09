use serde::Deserialize;

mod simple_fonts;
use simple_fonts::SimpleFont;

mod composite_fonts;
pub use composite_fonts::{CIDFontTypeTransient, Type0Transient, WElement};

mod descriptors;
pub use descriptors::{FontDescriptor, FontFlags};

use crate::{data::Rectangle, parsers::Extract, serde::extract_deserialize};

pub trait FontBehavior {
    fn convert(&self, string_input: &[u8]) -> (String, Rectangle);
    fn decode(&self, input: &[u8]) -> impl Iterator<Item = char>;
    fn width(&self, character: u8) -> u32;
}

#[derive(Debug, PartialEq, Deserialize, Clone)]
#[serde(tag = "Subtype", rename_all = "PascalCase")]
pub enum Font {
    Type0(Type0Transient),
    Type1(SimpleFont),
    MMType1(SimpleFont),
    TrueType(SimpleFont),
    // TODO: add Type3 font dict
    Type3,
    CIDFontType0(CIDFontTypeTransient),
    CIDFontType2(CIDFontTypeTransient),
}

impl Extract<'_> for Font {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        extract_deserialize(input)
    }
}
