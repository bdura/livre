//! Simple fonts with built-in glyph metrics: `Type1`, `MMType1` and `TrueType` (ISO 32000-2,
//! 9.6.2 to 9.6.3).

use livre_font_data::StandardFont;
use winnow::ModalResult;

use super::{
    decoder::{pop_to_unicode, Decoder},
    encoding::{EncodingSpec, Flags},
    widths::{Widths, WidthsDict},
    Code, GlyphWidth,
};
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
    /// The widths of the codes, from the font dictionary or the standard metrics.
    pub widths: SimpleWidths,
    /// How the font's codes turn into text.
    pub decoder: Decoder,
}

/// Where a simple font's widths come from.
#[derive(Debug, Clone, PartialEq)]
pub enum SimpleWidths {
    /// `/FirstChar`, `/Widths` and the descriptor's `/MissingWidth`.
    Table(Widths),
    /// No `/Widths`, and a `/BaseFont` naming one of the standard 14: the width of each code's
    /// glyph name in the font's metrics, else `missing`.
    Standard {
        font: StandardFont,
        missing: GlyphWidth,
    },
}

impl SimpleFont {
    /// The width of `code`'s glyph, in thousandths of text space.
    pub fn width(&self, code: Code) -> GlyphWidth {
        match &self.widths {
            SimpleWidths::Table(widths) => widths.width(code),
            SimpleWidths::Standard { font, missing } => self
                .decoder
                .glyph_name(code)
                .and_then(|name| font.width(name.as_str()))
                .map_or(*missing, |width| GlyphWidth(f32::from(width))),
        }
    }
}

#[derive(Debug, BuildFromRawDict)]
struct SimpleFontDict {
    base_font: Name,
    encoding: Option<EncodingSpec>,
    #[livre(flatten)]
    widths: WidthsDict,
}

/// Reads `/BaseFont`, `/Encoding`, `/ToUnicode`, `/FirstChar`, `/Widths` and the descriptor's
/// `/MissingWidth` and `/Flags`.
///
/// A font without `/Widths` takes the standard metrics if its `/BaseFont` names one of the
/// standard 14; otherwise it warns and gets the missing width for every code. An unreadable
/// `/ToUnicode` warns and is ignored.
impl BuildFromRawDict for SimpleFont {
    fn build_from_raw_dict<B>(dict: &mut RawDict<'_>, builder: &B) -> ModalResult<Self>
    where
        B: Builder,
    {
        let SimpleFontDict {
            base_font,
            encoding,
            widths,
        } = SimpleFontDict::build_from_raw_dict(dict, builder)?;
        let to_unicode = pop_to_unicode(dict, builder, &base_font);

        let (flags, missing) = widths
            .font_descriptor
            .as_ref()
            .map_or((Flags::default(), 0.0), |d| (d.flags, d.missing_width));
        let decoder = Decoder::new(
            encoding.as_ref(),
            &base_font,
            flags,
            to_unicode.as_deref(),
            true,
        );

        let standard = StandardFont::from_base_font(&String::from_utf8_lossy(&base_font));
        let widths = match (&widths.widths, standard) {
            (None, Some(font)) => SimpleWidths::Standard {
                font,
                missing: GlyphWidth(missing),
            },
            (None, None) => {
                tracing::warn!(
                    "Font {base_font:?} has no `/Widths`: every glyph gets the missing width"
                );
                SimpleWidths::Table(widths.into())
            }
            (Some(_), _) => SimpleWidths::Table(widths.into()),
        };

        Ok(Self {
            base_font,
            widths,
            decoder,
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
        tests::{font, font_with, HELVETICA_STANDARD},
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
    #[tracing_test::traced_test]
    fn no_widths_outside_the_standard_14_gives_the_missing_width() {
        let font = font(b"<</Type/Font/Subtype/TrueType/BaseFont/Calibri>>");

        let Font::Simple(SimpleFont { widths, .. }) = &font else {
            panic!("not a simple font: {font:?}");
        };
        assert!(matches!(widths, SimpleWidths::Table(table) if table.is_empty()));
        for code in [0, 32, 65, 255] {
            assert_eq!(width(&font, code), GlyphWidth(0.0));
        }
        assert!(logs_contain("has no `/Widths`"));
    }

    #[tracing_test::traced_test]
    #[rstest]
    #[case(65, 667.0)]
    #[case(32, 278.0)]
    fn standard_font_without_widths_takes_the_afm(#[case] code: u8, #[case] expected: f32) {
        let font = font(HELVETICA_STANDARD);

        assert_eq!(width(&font, code), GlyphWidth(expected));
        assert!(!logs_contain("has no `/Widths`"));
    }

    #[test]
    fn standard_font_without_widths_through_its_encoding() {
        let font = font(b"<</Type/Font/Subtype/Type1/BaseFont/Symbol>>");

        assert_eq!(font.width(Code::one_byte(0x61)), GlyphWidth(631.0));
    }

    #[test]
    fn standard_font_code_without_glyph_name_gets_the_missing_width() {
        let objects: HashMap<_, &BStr> = [(
            (5, 0).into(),
            b"5 0 obj\n<</Type/FontDescriptor/MissingWidth 250>>\nendobj"
                .as_slice()
                .as_ref(),
        )]
        .into();

        let font = font_with(
            b"<</Type/Font/Subtype/Type1/BaseFont/Helvetica/FontDescriptor 5 0 R>>",
            &objects,
        );

        assert_eq!(width(&font, 65), GlyphWidth(667.0));
        // `StandardEncoding` has no glyph at code 0.
        assert_eq!(width(&font, 0), GlyphWidth(250.0));
    }
}
