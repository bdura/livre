//! State for more elaborate content objects.
//!
//! Livre tracks the [`GraphicsState`] (the current transformation matrix and its `q`/`Q`
//! stack) between text objects, and a [`TextObject`] for the text operators inside
//! `BT`/`ET`. Nothing else of the graphics state is modelled yet.

mod graphics;
mod matrix;
mod text;

pub use graphics::GraphicsState;
pub use matrix::Matrix;
pub use text::{
    parse_text_object, Font, RenderingMode, TextObject, TextObjectStream, TextStateParameters,
};
