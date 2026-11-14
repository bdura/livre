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
pub use space::{DefaultUser, Displacement, Point, Text, User};
pub use text::{
    Font, RenderingMode, Run, TextObject, TextObjectStream, TextObjects, TextStateParameters,
};
