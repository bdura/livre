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

/// A point in default user space units.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Tsify)]
pub struct Point {
    pub x: f32,
    pub y: f32,
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
#[tsify(missing_as_null)]
pub struct Run {
    /// Where the string starts, on its baseline: the point `(0, rise)` of text space through
    /// the text matrix and the CTM in force at `BT`.
    pub start: Point,
    /// Where the string ends: the same point once the text matrix has advanced past every
    /// glyph. Composite fonts advance by spacing only, so their runs end short. Its `y`
    /// differs from `start`'s only under a rotated or skewed matrix.
    pub end: Point,
    /// Font resource name, lossily decoded as UTF-8.
    pub font: String,
    /// How the font is read, by its `/Subtype`; `None` when no font is selected.
    pub font_kind: Option<FontKind>,
    /// The font's `/BaseFont`, lossily decoded as UTF-8; `None` for a `Type3` font, which
    /// has none, or when no font is selected.
    pub base_font: Option<String>,
    /// Font size from `Tf`, in text space units: the scale of the text matrix is not applied.
    pub size: f32,
    /// The decoded text, through the font's `/ToUnicode` or encoding: the characters'
    /// texts concatenated, U+FFFD in place of each code with no text.
    pub text: String,
    /// The string's codes as lowercase hex, each written at its byte length: two digits for
    /// a one-byte code, four for a two-byte one.
    pub bytes: String,
    /// One per character code, in string order.
    pub characters: Vec<Character>,
}

/// One character code of a shown string, and the glyph it places.
#[derive(Debug, Clone, PartialEq, Serialize, Tsify)]
#[tsify(missing_as_null)]
pub struct Character {
    /// The code's numeric value.
    pub code: u32,
    /// Where the glyph starts, on the baseline, in default user space.
    pub start: Point,
    /// Where the glyph ends: its start advanced by its width and the spacing after it.
    pub end: Point,
    /// What the code stands for; `None` when neither `/ToUnicode` nor the encoding maps it.
    pub text: Option<String>,
}

/// The kind of font a run is shown in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Tsify)]
#[serde(rename_all = "kebab-case")]
pub enum FontKind {
    /// `Type1`, `MMType1` or `TrueType`.
    Simple,
    /// `Type3`.
    Type3,
    /// `Type0`: its glyphs are given zero width.
    Composite,
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
    /// Decoding and concatenating a page's content streams, and resolving its fonts.
    Content,
    /// Parsing a page's content stream into operators.
    Operators,
    /// Assembling a text object from its operators.
    TextObject,
}
