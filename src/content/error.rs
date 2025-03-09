use thiserror::Error;

use super::operators::TextShowingOperator;

#[derive(Error, Debug)]
pub enum ContentError {
    #[error("Got a text showing operator before setting the font and font size: {0:?}")]
    UnexpectedTextShowingOperator(TextShowingOperator),
    #[error("`BT` tag was not closed")]
    IncompleteTextObject,
}

pub type Result<T, E = ContentError> = std::result::Result<T, E>;
