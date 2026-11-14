//! `Type3` fonts, whose glyphs are content streams (ISO 32000-2, 9.6.4).

use winnow::ModalResult;

use super::widths::{Widths, WidthsDict};
use crate::{
    content::{Glyph, Matrix, Text},
    extraction::RawDict,
    follow_refs::{BuildFromRawDict, Builder},
};

/// A `Type3` font: one byte per code, widths in the glyph space its `/FontMatrix` defines.
#[derive(Debug, Clone, PartialEq)]
pub struct Type3Font {
    /// The `/FontMatrix`, mapping glyph space to text space.
    pub font_matrix: Matrix<Glyph, Text>,
    /// The widths of the codes, from `/FirstChar`, `/Widths` and `/MissingWidth`.
    pub widths: Widths,
}

#[derive(Debug, BuildFromRawDict)]
struct Type3FontDict {
    font_matrix: [f32; 6],
    #[livre(flatten)]
    widths: WidthsDict,
}

impl BuildFromRawDict for Type3Font {
    fn build_from_raw_dict<B>(dict: &mut RawDict<'_>, builder: &B) -> ModalResult<Self>
    where
        B: Builder,
    {
        let Type3FontDict {
            font_matrix: [a, b, c, d, e, f],
            widths,
        } = Type3FontDict::build_from_raw_dict(dict, builder)?;

        Ok(Self {
            font_matrix: Matrix::new(a, b, c, d, e, f),
            widths: widths.into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::font::{tests::font, Code, Font, GlyphWidth};

    #[test]
    fn font_matrix_and_widths() {
        let font = font(
            b"<</Type/Font/Subtype/Type3/FontMatrix[0.01 0 0 0.01 0 0]/FirstChar 97/Widths[50]>>",
        );

        assert!(matches!(font, Font::Type3(_)), "{font:?}");
        assert_eq!(
            font.glyph_to_text(),
            Matrix::new(0.01, 0.0, 0.0, 0.01, 0.0, 0.0)
        );
        assert_eq!(font.width(Code::one_byte(97)), GlyphWidth(50.0));
    }
}
