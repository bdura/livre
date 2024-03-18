//! Every possible errors.

use std::convert::Infallible;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExtractionError {
    #[error("Key {0} not found")]
    KeyNotFound(String),
    #[error("unknown error.")]
    Unknown,
}

pub type Result<T, E = ExtractionError> = std::result::Result<T, E>;

impl From<Infallible> for ExtractionError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}
