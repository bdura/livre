//! Glyph widths of fonts with one-byte codes, from `/FirstChar`, `/Widths` and the font
//! descriptor's `/MissingWidth` (ISO 32000-2, 9.6.2, Table 109, and 9.8, Table 120).

use winnow::ModalResult;

use crate::{
    extraction::{RawDict, Reference, Stream},
    follow_refs::{BuildFromRawDict, Builder},
};

use super::{encoding::Flags, Code};

/// A glyph's horizontal displacement, in [`Glyph`](crate::content::Glyph) space units:
/// thousandths of text space for `Type1` and `TrueType`, `/FontMatrix` units for `Type3`.
///
/// Only the font's glyph-to-text matrix turns it into a text space length.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GlyphWidth(pub f32);

/// The width table of a font whose codes are one byte long.
///
/// Codes from `first_char` on index the table; any code outside it gets the `missing` width.
#[derive(Debug, Clone, PartialEq)]
pub struct Widths {
    first_char: u8,
    widths: Vec<GlyphWidth>,
    missing: GlyphWidth,
}

impl Widths {
    /// Assemble the table from the font dictionary's `/FirstChar` and `/Widths` and the
    /// descriptor's `/MissingWidth`.
    ///
    /// Odd tables are clamped with a warning rather than refused, since positions remain
    /// recoverable: a `first_char` outside `0..=255` drops the table, and entries past code
    /// 255 are ignored. `widths` absent gives an empty table, where every code is `missing`.
    pub fn from_parts(first_char: i64, widths: Option<Vec<f32>>, missing_width: f32) -> Self {
        let missing = GlyphWidth(missing_width);
        let widths = widths.unwrap_or_default();

        let Ok(first_char) = u8::try_from(first_char) else {
            tracing::warn!("`/FirstChar {first_char}` outside 0..=255: `/Widths` ignored");
            return Self {
                first_char: 0,
                widths: Vec::new(),
                missing,
            };
        };

        let capacity = 256 - usize::from(first_char);
        let mut widths: Vec<GlyphWidth> = widths.into_iter().map(GlyphWidth).collect();
        if widths.len() > capacity {
            tracing::warn!(
                "`/Widths` has {} entries from `/FirstChar {first_char}`: truncated to {capacity}",
                widths.len()
            );
            widths.truncate(capacity);
        }

        Self {
            first_char,
            widths,
            missing,
        }
    }

    /// The width of `code`: its table entry if it has one, the missing width otherwise.
    pub fn width(&self, code: Code) -> GlyphWidth {
        code.value()
            .checked_sub(u32::from(self.first_char))
            .and_then(|index| self.widths.get(usize::try_from(index).ok()?))
            .copied()
            .unwrap_or(self.missing)
    }

    /// Whether the table has no entry, so that every code gets the missing width.
    pub fn is_empty(&self) -> bool {
        self.widths.is_empty()
    }
}

/// The keys of a font dictionary that [`Widths`] is assembled from.
#[derive(Debug, BuildFromRawDict)]
pub(super) struct WidthsDict {
    #[livre(default)]
    pub first_char: i64,
    pub widths: Option<Vec<f32>>,
    pub font_descriptor: Option<FontDescriptor>,
}

impl From<WidthsDict> for Widths {
    fn from(
        WidthsDict {
            first_char,
            widths,
            font_descriptor,
        }: WidthsDict,
    ) -> Self {
        let missing_width = font_descriptor.map_or(0.0, |d| d.missing_width);
        Widths::from_parts(first_char, widths, missing_width)
    }
}

/// Reads `/FirstChar`, `/Widths` and `/FontDescriptor` from a font dictionary, so that a
/// font can flatten its widths into its own fields.
impl BuildFromRawDict for Widths {
    fn build_from_raw_dict<B>(dict: &mut RawDict<'_>, builder: &B) -> ModalResult<Self>
    where
        B: Builder,
    {
        WidthsDict::build_from_raw_dict(dict, builder).map(Into::into)
    }
}

/// The font descriptor (ISO 32000-2, 9.8), reduced to the keys the advance and the encoding
/// need.
#[derive(Debug, Clone, PartialEq, BuildFromRawDict)]
pub struct FontDescriptor {
    /// The width of codes absent from `/Widths`, in glyph space units. Defaults to 0.
    #[livre(default)]
    pub missing_width: f32,
    /// `/Flags`, which tell a symbolic font from a Latin one. Defaults to none set.
    #[livre(default)]
    pub flags: Flags,
    /// `/FontFile`, the embedded Type 1 program, left unresolved: only a symbolic font
    /// without a predefined encoding reads it, for its built-in encoding. Reading it for
    /// every font would inflate each font program whether or not it is needed.
    pub font_file: Option<Reference<Stream<()>>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn one_byte(value: u8) -> Code {
        Code::one_byte(value)
    }

    #[rstest]
    #[case(65, 667.0)]
    #[case(67, 722.0)]
    #[case(68, 250.0)]
    #[case(32, 250.0)]
    fn width_in_and_out_of_range(#[case] code: u8, #[case] expected: f32) {
        let widths = Widths::from_parts(65, Some(vec![667.0, 667.0, 722.0]), 250.0);
        assert_eq!(widths.width(one_byte(code)), GlyphWidth(expected));
    }

    #[test]
    fn no_widths_gives_missing_everywhere() {
        let widths = Widths::from_parts(0, None, 250.0);
        assert!(widths.is_empty());
        assert_eq!(widths.width(one_byte(0)), GlyphWidth(250.0));
        assert_eq!(widths.width(one_byte(255)), GlyphWidth(250.0));
    }

    #[rstest]
    #[case(-1)]
    #[case(256)]
    fn first_char_out_of_range_drops_the_table(#[case] first_char: i64) {
        let widths = Widths::from_parts(first_char, Some(vec![500.0]), 0.0);
        assert!(widths.is_empty());
    }

    #[test]
    fn widths_past_255_are_truncated() {
        let widths = Widths::from_parts(254, Some(vec![1.0, 2.0, 3.0]), 0.0);
        assert_eq!(widths.width(one_byte(255)), GlyphWidth(2.0));
        assert_eq!(widths.widths.len(), 2);
    }
}
