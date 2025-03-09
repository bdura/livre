//! State for more elaborate content objects.
//!
//! For now, livre only supports text objects.

mod text;

pub use text::{parse_text_object, RenderingMode, TextMatrix, TextObject, TextStateParameters};
