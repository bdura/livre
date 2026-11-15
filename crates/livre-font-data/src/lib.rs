//! Font data for Livre: the [Adobe Glyph List] and the metrics of the
//! fourteen standard Type 1 fonts (ISO 32000-1:2008, §9.6.2.2).
//!
//! The upstream files are vendored unmodified under `data/`; the lookup
//! tables in `tables/` are derived from those files by this crate's tests and
//! committed, so the crate has no build script and no dependencies.
//!
//! Three entry points:
//!
//! - [`glyph_list::text`], the text a glyph name stands for:
//!
//!   ```
//!   use livre_font_data::glyph_list;
//!   assert_eq!(glyph_list::text("Adieresis"), Some("Ä"));
//!   assert_eq!(glyph_list::text("ffi"), Some("\u{FB03}"));
//!   ```
//!
//! - [`StandardFont::from_base_font`], the standard font a `/BaseFont` name
//!   denotes, usual aliases included:
//!
//!   ```
//!   use livre_font_data::StandardFont;
//!   assert_eq!(
//!       StandardFont::from_base_font("TimesNewRomanPS-BoldItalicMT"),
//!       Some(StandardFont::TimesBoldItalic),
//!   );
//!   ```
//!
//! - [`StandardFont::width`], the advance width of a glyph in thousandths of
//!   an em:
//!
//!   ```
//!   use livre_font_data::StandardFont;
//!   assert_eq!(StandardFont::Helvetica.width("A"), Some(667));
//!   ```
//!
//! [Adobe Glyph List]: https://github.com/adobe-type-tools/agl-aglfn

#![forbid(unsafe_code)]

pub mod glyph_list;
pub mod standard_font;
mod table;

pub use standard_font::StandardFont;

#[cfg(test)]
mod generate;
