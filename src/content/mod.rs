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
//! Since our primary goal is to be a text extraction library, we focus on text operators,
//! although we plan to expand this to other types of content in the future.
//! With that in mind, the main entry point for content parsing is the [`parse_text_object`],
//! which simplifies the process of extracting text from a content stream - and just skips
//! any other type of operator.

mod error;
pub mod operators;
mod state;

pub use error::ContentError;
pub use state::parse_text_object;
