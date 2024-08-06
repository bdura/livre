use enum_dispatch::enum_dispatch;

use serde::Deserialize;

mod type1;
use type1::Type1;

mod descriptors;
pub use descriptors::{FontDescriptor, FontFlags};

use crate::{data::Rectangle, parsers::Extract, serde::extract_deserialize};

#[enum_dispatch]
pub trait FontBehavior {
    /// The font should group bytes into character codes
    fn convert(&self, string_input: &[u8]) -> (String, Rectangle);
}

#[derive(Debug, PartialEq, Deserialize, Clone)]
#[serde(tag = "Subtype")]
pub enum Font {
    Type0,
    Type1(Type1),
    MMType1, //(Type1),
    TrueType, //(Type1),
    Type3,
    CIDFont,
}

impl Extract<'_> for Font {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        extract_deserialize(input)
    }
}
