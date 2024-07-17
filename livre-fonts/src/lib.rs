use enum_dispatch::enum_dispatch;

mod descriptors;
mod type1;

pub use descriptors::{FontDescriptor, FontFlags};
use livre_data::Rectangle;

#[enum_dispatch]
pub trait FontBehavior {
    /// The font should group bytes into character codes
    fn convert(&self, string_input: &[u8]) -> (String, Rectangle);
}

pub enum Font {
    // Type0,
    Type1,
    // Type3,
    // TrueType,
    // CIDFont,
}
