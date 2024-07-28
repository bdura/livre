use enum_dispatch::enum_dispatch;

mod descriptors;
mod type1;

pub use type1::Type1FontDict;

pub use descriptors::{FontDescriptor, FontFlags};
use livre_data::Rectangle;
use serde::Deserialize;

#[enum_dispatch]
pub trait FontBehavior {
    /// The font should group bytes into character codes
    fn convert(&self, string_input: &[u8]) -> (String, Rectangle);
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(tag = "Subtype")]
pub enum Font {
    // Type0,
    Type1(Type1FontDict),
    MMType1(Type1FontDict),
    // Type3,
    // TrueType,
    // CIDFont,
}
