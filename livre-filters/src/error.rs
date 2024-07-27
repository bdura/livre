//! Every possible errors.

use std::convert::Infallible;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum FilterError {
    #[error("Filter decode error")]
    FilterDecode(#[from] std::io::Error),
    // #[error("unknown error.")]
    // Unknown,
}

pub type Result<T, E = FilterError> = std::result::Result<T, E>;

impl From<Infallible> for FilterError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}
