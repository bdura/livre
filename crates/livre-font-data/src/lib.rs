//! Font data for Livre: the [Adobe Glyph List] and the metrics of the
//! fourteen standard Type 1 fonts (ISO 32000-1:2008, §9.6.2.2).
//!
//! The upstream files are vendored unmodified under `data/`; the lookup
//! tables beside them are derived from those files by this crate's tests and
//! committed, so the crate has no build script and no dependencies.
//!
//! [Adobe Glyph List]: https://github.com/adobe-type-tools/agl-aglfn

#![forbid(unsafe_code)]
