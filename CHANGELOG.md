# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
