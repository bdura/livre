//! `Type3` fonts, whose glyphs are content streams (ISO 32000-2, 9.6.4).

use super::widths::Widths;
use crate::{
    content::{Glyph, Matrix, Text},
    follow_refs::BuildFromRawDict,
};

/// A `Type3` font: one byte per code, widths in the glyph space its `/FontMatrix` defines.
#[derive(Debug, Clone, PartialEq, BuildFromRawDict)]
pub struct Type3Font {
    /// The `/FontMatrix`, mapping glyph space to text space.
    #[livre(from = [f32; 6])]
    pub font_matrix: Matrix<Glyph, Text>,
    /// The widths of the codes, from `/FirstChar`, `/Widths` and `/MissingWidth`.
    #[livre(flatten)]
    pub widths: Widths,
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
