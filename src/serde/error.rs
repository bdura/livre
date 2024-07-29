use std::fmt::Display;

use serde::{de, ser};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

// This is a bare-bones implementation. A real library would provide additional
// information in its error type, for example the line and column at which the
// error occurred, the byte offset into the input, or the current key being
// processed.
#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Message(String),
    /// The Eof error _may_ trigger a buffer read...
    #[error("unexpected end of input")]
    Eof,
    #[error("wrong syntax")]
    Syntax,
    #[error("expected PDF special token")]
    ExpectedPdfToken,
    #[error("expected boolean")]
    ExpectedBoolean,
    #[error("expected integer")]
    ExpectedInteger,
    #[error("expected float")]
    ExpectedFloat,
    #[error("expected sting")]
    ExpectedString,
    #[error("expected unit")]
    ExpectedNull,
    #[error("expected name")]
    ExpectedName,
    #[error("expected array")]
    ExpectedArray,
    #[error("expected array end")]
    ExpectedArrayEnd,
    #[error("expected map")]
    ExpectedMap,
    #[error("expected map end")]
    ExpectedMapEnd,
    #[error("expected enum")]
    ExpectedEnum,
    #[error("detected trailing characters")]
    TrailingCharacters,
    // #[error("unknown error")]
    // Unknown,
}

impl Error {
    pub fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Self::custom(msg)
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Self::custom(msg)
    }
}
