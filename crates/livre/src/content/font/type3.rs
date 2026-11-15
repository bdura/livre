//! `Type3` fonts, whose glyphs are content streams (ISO 32000-2, 9.6.4).

use winnow::ModalResult;

use super::{
    decoder::{pop_to_unicode, Decoder},
    encoding::{EncodingSpec, Flags},
    widths::Widths,
};
use crate::{
    content::{Glyph, Matrix, Text},
    extraction::{Name, RawDict},
    follow_refs::{BuildFromRawDict, Builder},
};

/// A `Type3` font: one byte per code, widths in the glyph space its `/FontMatrix` defines.
#[derive(Debug, Clone, PartialEq)]
pub struct Type3Font {
    /// The `/FontMatrix`, mapping glyph space to text space.
    pub font_matrix: Matrix<Glyph, Text>,
    /// The widths of the codes, from `/FirstChar`, `/Widths` and `/MissingWidth`.
    pub widths: Widths,
    /// How the font's codes turn into text: `/ToUnicode`, then the `/Differences` of its
    /// `/Encoding`, with no built-in table under them.
    pub decoder: Decoder,
}

#[derive(Debug, BuildFromRawDict)]
struct Type3FontDict {
    #[livre(from = [f32; 6])]
    font_matrix: Matrix<Glyph, Text>,
    #[livre(flatten)]
    widths: Widths,
    encoding: Option<EncodingSpec>,
}

/// Reads `/FontMatrix`, the widths, `/Encoding` and `/ToUnicode`. `/Encoding` is required:
/// without it, the font warns and only `/ToUnicode` can decode its codes.
impl BuildFromRawDict for Type3Font {
    fn build_from_raw_dict<B>(dict: &mut RawDict<'_>, builder: &B) -> ModalResult<Self>
    where
        B: Builder,
    {
        let Type3FontDict {
            font_matrix,
            widths,
            encoding,
        } = Type3FontDict::build_from_raw_dict(dict, builder)?;

        // `Type3` fonts have no `/BaseFont`: warnings name the subtype instead.
        let name = Name::from("Type3");
        if encoding.is_none() {
            tracing::warn!("Font Type3 has no `/Encoding`: its codes have no glyph names");
        }
        let to_unicode = pop_to_unicode(dict, builder, &name);
        let decoder = Decoder::new(
            encoding.as_ref(),
            &name,
            Flags::default(),
            to_unicode.as_deref(),
            false,
        );

        Ok(Self {
            font_matrix,
            widths,
            decoder,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::font::{
        tests::{bfchar, decode, font, font_with_stream},
        Code, Font, GlyphWidth,
    };

    #[test]
    fn font_matrix_and_widths() {
        let font = font(
            b"<</Type/Font/Subtype/Type3/FontMatrix[0.01 0 0 0.01 0 0]/FirstChar 97/Widths[50]/Encoding<</Differences[97/a]>>>>",
        );

        assert!(matches!(font, Font::Type3(_)), "{font:?}");
        assert_eq!(
            font.glyph_to_text(),
            Matrix::new(0.01, 0.0, 0.0, 0.01, 0.0, 0.0)
        );
        assert_eq!(font.width(Code::one_byte(97)), GlyphWidth(50.0));
    }

    const SQUARES: &[u8] = b"<</Type/Font/Subtype/Type3/FontMatrix[0.001 0 0 0.001 0 0]\
        /FirstChar 97/Widths[500 500]/Encoding<</Differences[97 /whitesquare 98 /myglyph]>>";

    #[test]
    fn decodes_through_its_differences() {
        let font = font(&[SQUARES, b">>"].concat());

        assert_eq!(decode(&font, 97).as_deref(), Some("□"));
        assert_eq!(decode(&font, 98), None);
        // No built-in table under the differences.
        assert_eq!(decode(&font, 65), None);
    }

    #[test]
    fn to_unicode_names_what_the_differences_cannot() {
        let font = font_with_stream(
            &[SQUARES, b"/ToUnicode 9 0 R>>"].concat(),
            &bfchar("62", "25A0"),
        );

        assert_eq!(decode(&font, 97).as_deref(), Some("□"));
        assert_eq!(decode(&font, 98).as_deref(), Some("■"));
    }

    #[test]
    #[tracing_test::traced_test]
    fn without_encoding_warns_and_decodes_nothing() {
        let font = font(b"<</Type/Font/Subtype/Type3/FontMatrix[0.001 0 0 0.001 0 0]/FirstChar 97/Widths[500]>>");

        assert!(logs_contain("has no `/Encoding`"));
        for code in [0, 65, 97] {
            assert_eq!(decode(&font, code), None);
        }
    }
}
