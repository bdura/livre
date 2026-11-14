//! `Type0` composite fonts (ISO 32000-2, 9.7), as a placeholder: neither the CMap nor the
//! descendant's `/W` widths are read yet.

use crate::{extraction::Name, follow_refs::BuildFromRawDict};

/// A `Type0` font, reduced to its name. Its codes are read two bytes at a time, as
/// `Identity-H` does, and every glyph is zero wide.
#[derive(Debug, Clone, PartialEq, BuildFromRawDict)]
pub struct CompositeFont {
    /// The font's name, `/BaseFont`.
    pub base_font: Name,
}

#[cfg(test)]
mod tests {
    use crate::{
        content::font::{tests::font, Code, Font, GlyphWidth},
        extraction::PDFString,
    };

    fn calibri() -> Font {
        font(b"<</Type/Font/Subtype/Type0/BaseFont/ABCDEF+Calibri/DescendantFonts[6 0 R]/Encoding/Identity-H>>")
    }

    #[test]
    fn composite_with_zero_widths() {
        let font = calibri();

        assert!(matches!(font, Font::Composite(_)), "{font:?}");
        for code in [
            Code::two_bytes(0x41),
            Code::two_bytes(0),
            Code::one_byte(32),
        ] {
            assert_eq!(font.width(code), GlyphWidth(0.0));
        }
    }

    #[test]
    fn two_bytes_per_code() {
        let string = PDFString(b"\x00\x41\x00\x42".to_vec());
        let codes: Vec<_> = calibri().codes(&string).collect();

        assert_eq!(codes, [Code::two_bytes(0x41), Code::two_bytes(0x42)]);
    }

    #[test]
    fn odd_trailing_byte_is_dropped() {
        // A warning is logged as well; no subscriber captures it in the tests.
        let string = PDFString(b"\x00\x41\x00".to_vec());
        let codes: Vec<_> = calibri().codes(&string).collect();

        assert_eq!(codes, [Code::two_bytes(0x41)]);
    }
}
