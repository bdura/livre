# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `livre::VERSION`: the crate version, as published
- Font dictionaries, `livre::content::Font`: simple fonts (`Type1`, `MMType1`, `TrueType`) with their `/Widths` and descriptor's `/MissingWidth`, `Type3` fonts with their `/FontMatrix`, and `Type0` fonts as a placeholder whose glyphs have zero width. `Font::codes` splits a shown string into character codes
- `Fonts`, a page's font resources by name with every reference resolved, and `Page::fonts` to build it
- `Run::end`: where the shown string ends, once the text matrix has advanced past each of its glyphs

### Fixed

- Apply `/DecodeParms` predictors after `FlateDecode`: PNG (10 to 15) and TIFF (2, 8 bits per component), so cross-reference streams written with `/Predictor 12` decode correctly
- Read every field of a cross-reference stream row: free and uncompressed entries now consume their whole `/W` row, and a zero-width type field defaults to type 1
- Scale `TJ` offsets by the font size and horizontal scaling, as the spec requires: they were applied in raw thousandths of a unit
- Keep the text state parameters in the graphics state: a `Tf` (or any text state operator) outside `BT`/`ET` applies to the text objects that follow, the font carries from one text object to the next, and `q`/`Q` save and restore it with the CTM. A text object that shows text before its own `Tf` no longer fails
- Advance the text matrix past every shown glyph by its width, character spacing, word spacing (on single-byte code 32 only) and horizontal scaling, as ISO 32000-2 9.4.4 specifies: every string of a line was reported at the line's start, and `TJ` offsets accumulated from there
- Track the line matrix: `Td`, `TD` and `T*` move from the start of the current line, not from where the last string ended
- Apply the text rise `Ts` to the reported positions
- Accept a page whose `/Resources` has no `/Font` entry: it was rejected
- Inherit `/Resources` from the parent `/Pages` nodes when a page has none of its own

### Changed

- Raise the minimum supported Rust version to 1.85 (required by trybuild's dependencies)
- Replace the free function `parse_text_object` with `TextObjects`, which owns a content stream's operators and graphics state and lends out its text objects one at a time (it is not an `Iterator`): `while let Some(stream) = objects.next_object()? { … }`. `GraphicsState` is no longer public
- Text object streams yield `Run { position, text }`: the position as a `Point<DefaultUser>`, the shown string as its raw `PDFString` (`text.decode()` for the text)
- Type matrices and points by coordinate space: `Matrix<From, To>` and `Point<Space>` over the `Text`, `User` and `DefaultUser` markers, so composing mismatched matrices, or applying a matrix to a point of another space, does not compile
- `TextObject::move_to(x, y)` becomes `move_by(offset)`, taking a `Displacement<Text>`: the move is relative to the start of the current line, as `Td` is. The new `set_matrix` sets both the text and the line matrix, as `Tm` does
- `ContentError::NoFontSelected`, when text is shown with no font selected, replaces `UnexpectedTextShowingOperator`; a text operator outside `BT`/`ET` is skipped with a warning
- `TextObjects::new(ops, &fonts)` takes the page's `Fonts`, and `Tf` selects among them: the selected font is a `SelectedFont` borrowing the `Font`, with its size as a `FontSize`. A `Tf` naming a font absent from the resources is `ContentError::UnknownFont`

## [0.5.0](https://github.com/bdura/livre/compare/livre-v0.4.0...livre-v0.5.0) - 2025-03-09

### Added

- Page content extraction ([#41](https://github.com/bdura/livre/pull/41))

  - efficient-ish extraction of PDF operators
  - iteration over text objects

### Other

- update readme ([#43](https://github.com/bdura/livre/pull/43))

## [0.4.0](https://github.com/bdura/livre/compare/livre-v0.3.0...livre-v0.4.0) - 2025-02-16

### Added

- Page extraction & iteration (#39)

## [0.3.0](https://github.com/bdura/livre/compare/livre-v0.2.0...livre-v0.3.0) - 2025-01-20

### Added

- owned builder (#35)
- object stream (#34)
- implement `Build` for `Stream`s (#33)
- implement Parser for Builders (#29)
- cleaner `builder` module (#28)

### Fixed

- build containers (#32)

### Other

- update changelog (#38)
- add `release-plz` (#36)
- reorganise Build traits (#31)
- fix changelog...
- move structures into `extraction/special` (#30)
- filtering (#27)
- create a dedicated `references` submodule (#24)

### Changed

- Move PDF-specific datastructures from a dedicated `structures` module to `extraction/special`.
- Refactor the `Build` trait (and friends):
  - rename module `follow_refs`
  - drop blanket implementation for `Extract` types, allowing more flexibility - in particular
    with `Indirect` and `OptRef`
  - drop `Build` support for types that reference into the input data, simplifying reference-
    following traits.
  - add a `BuildFromRawDict` trait, that implements `Build`.
- Modify `LiteralString` to own its data

## [0.2.0] - 2024-12-26

### Added

- Declaration of the `Trailer` type to represent PDF trailers.
- Extraction facilities for cross-reference tables & trailer (regrouped as a single block),
  with support for cross-reference tables & streams. Such a block is described by the new
  `XRefTrailerBlock` type.

## [0.1.0] - 2024-12-23

### Added

- Low-level extraction utilities able to parse any PDF object in a type-safe manner,
  using three main traits:
  - `Extract`, which defines how a type can extract itself from a stream of bytes
  - `Builder`, which declares how an object can follow PDF references to build more
    complex objects
  - `Build`, which defines how a complex type can leverage a `Builder` object to
    build itself despite the presence of PDF references
- Extractable types include Rust primitive types as well as usual containers:
  - the unit type, which maps with the `null` PDF object
  - booleans
  - all integer and floating-point numbers
  - tuples, arrays, and vectors of extractable types
  - optional types
- ... as well as PDF-specific types, including:
  - PDF strings (literal & hexadecimal)
  - dictionaries
  - references (cf `Builder` & `Build` traits)
  - names
  - streams

[unreleased]: https://github.com/bdura/livre/compare/livre-v0.2.0...HEAD
[0.2.0]: https://github.com/bdura/livre/releases/tag/livre-v0.2.0
[0.1.0]: https://github.com/bdura/livre/releases/tag/v0.1.0
