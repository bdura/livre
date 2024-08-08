use enum_dispatch::enum_dispatch;

use serde::Deserialize;

mod simple_fonts;
use simple_fonts::Type1;

mod composite_fonts;
pub use composite_fonts::{Type0, WElement};

mod descriptors;
pub use descriptors::{FontDescriptor, FontFlags};

use crate::{data::Rectangle, objects::Object, parsers::Extract, serde::extract_deserialize};

// #[enum_dispatch]
pub trait FontBehavior {
    fn convert(&self, string_input: &[u8]) -> (String, Rectangle);
    fn decode(&self, input: &[u8]) -> impl Iterator<Item = char>;
    fn width(&self, character: u8) -> u32;
}

#[derive(Debug, PartialEq, Deserialize, Clone)]
#[serde(tag = "Subtype", rename_all = "PascalCase")]
pub enum Font {
    Type0(Type0),
    Type1(Type1),
    MMType1(Type1),
    // Type3,
    TrueType(Object),
    CIDFontType0(Object),
    #[serde(rename_all = "PascalCase")]
    CIDFontType2 {
        #[serde(rename = "DW", default)]
        default_width: Option<u16>,
        // #[serde(rename = "W", default)]
        // widths: Option<OptRef<Vec>>>,
    },
}

impl Extract<'_> for Font {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        extract_deserialize(input)
    }
}
