//! Simple fonts with built-in glyph metrics: `Type1`, `MMType1` and `TrueType` (ISO 32000-2,
//! 9.6.2 to 9.6.3).

use winnow::ModalResult;

use super::widths::{Widths, WidthsDict};
use crate::{
    extraction::{Name, RawDict},
    follow_refs::{BuildFromRawDict, Builder},
};

pub use super::widths::FontDescriptor;

/// A `Type1`, `MMType1` or `TrueType` font: one byte per code, widths in thousandths of
/// text space.
#[derive(Debug, Clone, PartialEq)]
pub struct SimpleFont {
    /// The font's PostScript name, `/BaseFont`.
    pub base_font: Name,
    /// The widths of the codes, from `/FirstChar`, `/Widths` and `/MissingWidth`.
    pub widths: Widths,
}

#[derive(Debug, BuildFromRawDict)]
struct SimpleFontDict {
    base_font: Name,
    #[livre(flatten)]
    widths: WidthsDict,
}

/// Reads `/BaseFont`, `/FirstChar`, `/Widths` and the descriptor's `/MissingWidth`. A font
/// without `/Widths`, as the standard 14 may be, warns and gets the missing width for every
/// code.
impl BuildFromRawDict for SimpleFont {
    fn build_from_raw_dict<B>(dict: &mut RawDict<'_>, builder: &B) -> ModalResult<Self>
    where
        B: Builder,
    {
        let SimpleFontDict { base_font, widths } =
            SimpleFontDict::build_from_raw_dict(dict, builder)?;

        if widths.widths.is_none() {
            tracing::warn!(
                "Font {base_font:?} has no `/Widths`: every glyph gets the missing width"
            );
        }

        Ok(Self {
            base_font,
            widths: widths.into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use rstest::rstest;
    use winnow::BStr;

    use super::*;
    use crate::content::font::{
        tests::{font, font_with},
        Code, Font, GlyphWidth,
    };

    fn width(font: &Font, code: u8) -> GlyphWidth {
        font.width(Code::one_byte(code))
    }

    #[rstest]
    #[case(65, 667.0)]
    #[case(68, 0.0)]
    #[case(32, 0.0)]
    fn widths_without_descriptor(#[case] code: u8, #[case] expected: f32) {
        let helvetica = font(
            b"<</Type/Font/Subtype/Type1/BaseFont/Helvetica/FirstChar 65/LastChar 67/Widths[667 667 722]>>",
        );

        let Font::Simple(SimpleFont { base_font, .. }) = &helvetica else {
            panic!("not a simple font: {helvetica:?}");
        };
        assert_eq!(base_font, &Name::from("Helvetica"));
        assert_eq!(width(&helvetica, code), GlyphWidth(expected));
    }

    #[test]
    fn missing_width_through_the_descriptor() {
        let objects: HashMap<_, &BStr> = [(
            (5, 0).into(),
            b"5 0 obj\n<</Type/FontDescriptor/MissingWidth 250>>\nendobj"
                .as_slice()
                .as_ref(),
        )]
        .into();

        let helvetica = font_with(
            b"<</Type/Font/Subtype/Type1/BaseFont/Helvetica/FirstChar 65/LastChar 67/Widths[667 667 722]/FontDescriptor 5 0 R>>",
            &objects,
        );

        assert_eq!(width(&helvetica, 65), GlyphWidth(667.0));
        assert_eq!(width(&helvetica, 68), GlyphWidth(250.0));
    }

    #[rstest]
    #[case(b"TrueType")]
    #[case(b"MMType1")]
    fn other_simple_subtypes(#[case] subtype: &[u8]) {
        let input = [
            b"<</Type/Font/Subtype/".as_slice(),
            subtype,
            b"/BaseFont/Arial/FirstChar 65/Widths[667]>>",
        ]
        .concat();

        let font = font(&input);
        assert!(matches!(font, Font::Simple(_)), "{font:?}");
        assert_eq!(width(&font, 65), GlyphWidth(667.0));
    }

    #[test]
    fn no_widths_gives_the_missing_width() {
        // A warning is logged as well; no subscriber captures it in the tests.
        let font = font(b"<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>");

        let Font::Simple(SimpleFont { widths, .. }) = &font else {
            panic!("not a simple font: {font:?}");
        };
        assert!(widths.is_empty());
        for code in [0, 32, 65, 255] {
            assert_eq!(width(&font, code), GlyphWidth(0.0));
        }
    }
}
