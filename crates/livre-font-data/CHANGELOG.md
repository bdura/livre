# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/bdura/livre/releases/tag/livre-font-data-v0.1.0) - 2026-11-15

### Added

- font data ([#67](https://github.com/bdura/livre/pull/67))

### Added

- The `livre-font-data` crate, vendoring the Adobe Glyph List and the Core 14 AFM
  files with their licence notices.
- `glyph_list::text`, the text a glyph name stands for by the Adobe Glyph List.
- `StandardFont`, the fourteen standard fonts, and `StandardFont::from_base_font`,
  which resolves a `/BaseFont` name and its usual aliases (Arial, Times New Roman,
  Courier New, with subset prefixes and style suffixes).
- `StandardFont::width`, glyph advance widths from the Core 14 AFM files.
