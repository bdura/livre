//! Every possible errors.

use std::{convert::Infallible, fmt::Display};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExtractionError {
    #[error("Key {0} not found")]
    KeyNotFound(String),
    #[error("unknown error. {0}")]
    Unknown(String),
}

impl ExtractionError {
    pub fn custom<T>(msg: T) -> Self where T: Display {
        Self::Unknown(msg.to_string())
    }
}

pub type Result<T, E = ExtractionError> = std::result::Result<T, E>;

impl From<Infallible> for ExtractionError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}
