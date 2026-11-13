//! The data handed to the demo page: a plain mirror of what livre extracted, shaped for
//! serialisation so that livre's own types never commit to a wire format.
//!
//! Field names serialise as written (snake_case). Coordinates are in default user space
//! units, with the origin at the bottom left of the page.
//!
//! Each type derives [`Tsify`], so wasm-bindgen declares a TypeScript interface of the
//! same name for the page to type-check against. An absent `Option` is `null` on both
//! sides.

use serde::Serialize;
use tsify::Tsify;

/// Everything livre extracted from one PDF.
#[derive(Debug, Clone, PartialEq, Serialize, Tsify)]
#[tsify(missing_as_null)]
pub struct Analysis {
    /// Version of the livre crate that ran the analysis.
    pub version: String,
    /// Set when the document or its page tree could not be read, in which case `pages`
    /// is empty.
    pub document: Option<StageError>,
    /// The pages in page tree order.
    pub pages: Vec<Page>,
}

/// One page of the document, with what livre extracted from its content.
#[derive(Debug, Clone, PartialEq, Serialize, Tsify)]
#[tsify(missing_as_null)]
pub struct Page {
    /// Zero-based position in the flattened page tree.
    pub index: usize,
    /// The `MediaBox`, inherited from the page tree if the page has none.
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

/// A rectangle by its corners, in default user space units.
#[derive(Debug, Clone, PartialEq, Serialize, Tsify)]
pub struct Rect {
    pub xmin: f32,
    pub ymin: f32,
    pub xmax: f32,
    pub ymax: f32,
}

/// One `BT` ... `ET` block.
#[derive(Debug, Clone, PartialEq, Serialize, Tsify)]
pub struct TextObject {
    /// The strings it shows, in content stream order.
    pub runs: Vec<Run>,
}

/// One shown string.
#[derive(Debug, Clone, PartialEq, Serialize, Tsify)]
pub struct Run {
    /// Start position, in user space: the translation of the text matrix times the CTM in
    /// force at `BT`. livre knows no glyph widths, so there is no extent.
    pub x: f32,
    /// Baseline of the start position.
    pub y: f32,
    /// Font resource name, lossily decoded as UTF-8.
    pub font: String,
    /// Font size from `Tf`, in text space units: the scale of the text matrix is not applied.
    pub size: f32,
    /// Best-effort decoding of `bytes`: UTF-16BE behind a BOM, Latin-1 otherwise.
    pub text: String,
    /// The raw string bytes, as lowercase hex.
    pub bytes: String,
}

/// Where and why extraction stopped.
#[derive(Debug, Clone, PartialEq, Serialize, Tsify)]
pub struct StageError {
    /// The step that failed.
    pub stage: Stage,
    /// The underlying error, `Debug`-formatted.
    pub message: String,
}

/// The step of the extraction that failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Tsify)]
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
