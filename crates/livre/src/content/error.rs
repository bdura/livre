use thiserror::Error;

/// Errors related to content parsing.
#[derive(Error, Debug)]
pub enum ContentError {
    /// Text was shown while the text state had no font: no `Tf` ran before it, neither in
    /// the text object nor earlier in the content stream.
    #[error("Got a text showing operator before any `Tf` selected a font")]
    NoFontSelected,
    #[error("`BT` tag was not closed")]
    IncompleteTextObject,
}

pub type Result<T, E = ContentError> = std::result::Result<T, E>;
