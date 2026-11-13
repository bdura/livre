//! The data handed to the demo page: a plain mirror of what livre extracted, shaped for
//! serialisation so that livre's own types never commit to a wire format.
//!
//! Field names serialise as written (snake_case). Coordinates are in default user space
//! units, with the origin at the bottom left of the page.

use serde::Serialize;

/// Everything livre extracted from one PDF.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Analysis {
    /// Version of the livre crate that ran the analysis.
    pub version: String,
    /// Set when the document or its page tree could not be read, in which case `pages`
    /// is empty.
    pub document: Option<StageError>,
    pub pages: Vec<Page>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Page {
    /// Zero-based position in the flattened page tree.
    pub index: usize,
    pub media_box: Rect,
    /// `true` when the page tree carries no `MediaBox` and `media_box` is US Letter.
    pub media_box_defaulted: bool,
    /// Size of a user space unit, in multiples of 1/72 inch.
    pub user_unit: f32,
    /// Text objects in content stream order, including those parsed before `error`.
    pub text_objects: Vec<TextObject>,
    /// The failure that stopped this page's extraction, if any.
    pub error: Option<StageError>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Rect {
    pub xmin: f32,
    pub ymin: f32,
    pub xmax: f32,
    pub ymax: f32,
}

/// One `BT` ... `ET` block.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TextObject {
    pub runs: Vec<Run>,
}

/// One shown string.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Run {
    /// Start position from the text matrix; livre knows no glyph widths, so no extent.
    pub x: f32,
    pub y: f32,
    /// Font resource name, lossily decoded as UTF-8.
    pub font: String,
    pub size: f32,
    /// Best-effort decoding of `bytes`: UTF-16BE behind a BOM, Latin-1 otherwise.
    pub text: String,
    /// The raw string bytes, as lowercase hex.
    pub bytes: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StageError {
    pub stage: Stage,
    /// The underlying error, `Debug`-formatted.
    pub message: String,
}

/// The step of the extraction that failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Stage {
    /// Reading the cross-reference chain and the catalog.
    Document,
    /// Flattening the page tree.
    Pages,
    /// Decoding and concatenating a page's content streams.
    Content,
    /// Parsing a page's content stream into operators.
    Operators,
    /// Assembling a text object from its operators.
    TextObject,
}
