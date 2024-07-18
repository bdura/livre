use std;
use std::fmt::{self, Display};

use livre_extraction::error::ExtractionError;
use serde::{de, ser};

pub type Result<T> = std::result::Result<T, Error>;

// This is a bare-bones implementation. A real library would provide additional
// information in its error type, for example the line and column at which the
// error occurred, the byte offset into the input, or the current key being
// processed.
#[derive(Debug)]
pub enum Error {
    // One or more variants that can be created by data structures through the
    // `ser::Error` and `de::Error` traits. For example the Serialize impl for
    // Mutex<T> might return an error because the mutex is poisoned, or the
    // Deserialize impl for a struct may return an error because a required
    // field is missing.
    Message(String),

    // Zero or more variants that can be created directly by the Serializer and
    // Deserializer without going through `ser::Error` and `de::Error`. These
    // are specific to the format, in this case JSON.
    Eof,
    Syntax,
    ExpectedBoolean,
    ExpectedInteger,
    ExpectedString,
    ExpectedNull,
    ExpectedArray,
    // ExpectedArrayComma,
    ExpectedArrayEnd,
    ExpectedMap,
    // ExpectedMapColon,
    // ExpectedMapComma,
    ExpectedMapEnd,
    ExpectedEnum,
    TrailingCharacters,
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Message(msg) => formatter.write_str(msg),
            Self::Eof => formatter.write_str("unexpected end of input"),
            Self::Syntax => formatter.write_str("wrong syntax"),
            Self::ExpectedBoolean => formatter.write_str("expected boolean"),
            Self::ExpectedInteger => formatter.write_str("expected integer"),
            Self::ExpectedString => formatter.write_str("expected sting"),
            Self::ExpectedNull => formatter.write_str("expected unit"),
            Self::ExpectedArray => formatter.write_str("expected array"),
            Self::ExpectedArrayEnd => formatter.write_str("expected array end"),
            Self::ExpectedMap => formatter.write_str("expected map"),
            Self::ExpectedMapEnd => formatter.write_str("expected map end"),
            Self::ExpectedEnum => formatter.write_str("expected enum"),
            Self::TrailingCharacters => formatter.write_str("detected trailing characters"),
        }
    }
}

impl std::error::Error for Error {}

impl From<ExtractionError> for Error {
    fn from(value: ExtractionError) -> Self {
        Error::Message(value.to_string())
    }
}
