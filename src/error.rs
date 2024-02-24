//! Every possible errors.

use std::{convert::Infallible, num::TryFromIntError};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParsingError {
    #[error("Unexpected type: expected {expected}, got {got}")]
    UnexpectedType {
        expected: &'static str,
        got: &'static str,
    },
    #[error("Key `{0}` not found")]
    KeyNotFound(String),
    #[error("Filter decode error")]
    FilterDecode(#[from] std::io::Error),
    #[error("Conversion error")]
    ConversionError,
}

pub type Result<T, E = ParsingError> = std::result::Result<T, E>;

impl From<Infallible> for ParsingError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

impl From<TryFromIntError> for ParsingError {
    fn from(_: TryFromIntError) -> Self {
        Self::ConversionError
    }
}
