//! Utilities for parsing and manipulating page content.
//!
//! In a PDF file, a page's content is contained within an list of
//! [stream objects](crate::extraction::Stream). Each stream object contains a sequence of
//! operators that are used to draw text, images, and other graphical elements on the page.
//!
//! Livre provides a set of utilities to iterate over these operators. The basic building
//! bloc is the [`Operator`](operators::Operator) type, which represents a single operator
//! in the content stream.
//!
//! Since our primary goal is to be a text extraction library, we focus on text operators and
//! on the graphics state operators that place them (`q`, `Q`, `cm`), although we plan to
//! expand this to other types of content in the future. With that in mind, the main entry
//! point for content parsing is [`TextObjects`]: it walks a page's operators, tracks the
//! graphics state (the CTM and the text state, through `q` and `Q`) across text objects,
//! yields each text object that shows text, and skips any other type of operator.

mod error;
pub mod operators;
mod state;

pub use error::ContentError;
pub use state::{
    DefaultUser, Displacement, Font, Glyph, Matrix, Point, RenderingMode, Run, Text, TextObject,
    TextObjectStream, TextObjects, TextStateParameters, User,
};
