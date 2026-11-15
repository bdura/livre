//! State for more elaborate content objects.
//!
//! [`TextObjects`] tracks the graphics state (the current transformation matrix, the text
//! state parameters and their `q`/`Q` stack) across a content stream, and a [`TextObject`]
//! the text operators inside `BT`/`ET`. Nothing else of the graphics state is modelled yet.

mod graphics;
mod matrix;
mod space;
mod text;

pub(crate) use graphics::GraphicsState;
pub use matrix::Matrix;
pub use space::{DefaultUser, Displacement, Glyph, Point, Text, User};
pub use text::{
    FontSize, RenderingMode, Run, SelectedFont, TextObject, TextObjectStream, TextObjects,
    TextStateParameters,
};
