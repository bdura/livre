# Roadmap

## Technical details

### Fonts

In PDFs, fonts are responsible for:

- grouping input bytes into character codes
- providing the spacing information for each glyph

These reponsibilities should be included in the [`FontBehavior`] trait.

Let's focus on `Type0` and `TrueType` fonts for now: those are the one present
in the two current examples.

## High-level features

- [ ] extraction by painted bloc (`TJ` operator)
- [ ] async support for multi-file
- [ ] Python bindings

## Low-level

- [ ] introduce typed references (type could be `Object` by default), to allow type-safe extraction without lookup.
