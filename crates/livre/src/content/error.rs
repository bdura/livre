use thiserror::Error;

use crate::extraction::Name;

/// Errors related to content parsing.
#[derive(Error, Debug)]
pub enum ContentError {
    /// Text was shown while the text state had no font: no `Tf` ran before it, neither in
    /// the text object nor earlier in the content stream.
    #[error("Got a text showing operator before any `Tf` selected a font")]
    NoFontSelected,
    /// A text object has no `ET`: the content stream ends inside it, either before it shows
    /// any text or after its stream ended (dropped, or out of operators) without reading
    /// the `ET`.
    #[error("`BT` tag was not closed")]
    IncompleteTextObject,
    /// A `Tf` named a font absent from the page's `/Resources /Font` map.
    #[error("`Tf` selects /{}, which is not among the page's font resources", String::from_utf8_lossy(.0))]
    UnknownFont(Name),
}

pub type Result<T, E = ContentError> = std::result::Result<T, E>;
