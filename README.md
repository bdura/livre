# Livre

Livre, pronounced [[livʁ]](https://en.wiktionary.org/wiki/file:fr-un_livre-fr-ouest.ogg)
(the french word for book) aims to be a type-safe utility for parsing PDF documents.

Please note that this is a **very** early release, primarily meant as a motivational
tool for me to work towards an actually working release in the near future.

This first version already provides a sufficient set of low-level utilities to
extract all PDF primitive types in a type-safe manner.

## Demo

The [browser demo](https://bdura.github.io/livre/) runs livre, compiled to
WebAssembly, on a PDF of your choosing and draws what it extracted over a pdf.js
render of the page, next to pdf.js's own text layer. Files stay in your browser:
nothing is uploaded. The overlay draws each string from where it starts to where
it ends, as livre reads them from simple fonts' widths. Composite fonts (`Type0`,
common in Word exports) are not read yet: their strings advance by the character
spacing only. See [`demo/README.md`](demo/README.md) to run it locally.

## Obtaining the PDF specification

Thankfully, the ISO standards defining the PDF specification can be downloaded free of
charge. Visit the [PDF Association's website](https://pdfa.org/resource/iso-32000-pdf/)
to get your version.

## Tentative roadmap

- [x] (`v0.1.0`) Low-level extraction utilities for primitive PDF types
- [x] (`v0.2.0`) Parser for cross-reference dictionary
- [x] (`v0.3.0`) Eager, owned instance of the `Builder` trait
- [x] (`v0.4.0`) Parser for the general PDF structure, allowing to iterate over
  pages and their content
- [x] (`v0.5.0`) Parser for pages' content operators & text content
- [ ] (`v0.6.0`) Parser for fonts definition - starting with "simple fonts".
  This step will allow iterating over characters and their actual position.
  - [x] Glyph widths of simple fonts, so each shown string has a start and an end
  - [x] Decoding character codes of simple fonts to text, with each
    character's position
  - [ ] Composite (`Type0`) fonts
