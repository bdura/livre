//! From a character code of a simple or `Type3` font to the text it stands for (ISO 32000-2,
//! 9.10.2).

use std::borrow::Cow;

use super::{
    encoding::{base_table, Differences, EncodingSpec},
    Code, Encoding, Flags, GlyphName, ToUnicode,
};
use crate::{
    extraction::{Name, RawDict, Stream},
    follow_refs::{Builder, Built},
};

/// How a simple or `Type3` font turns a code into text (ISO 32000-2, 9.10.2).
///
/// `/ToUnicode` wins; a code it does not map falls back to its glyph name, through the
/// encoding and the glyph list.
#[derive(Debug, Clone, PartialEq)]
pub struct Decoder {
    to_unicode: Option<ToUnicode>,
    encoding: Encoding,
}

impl Decoder {
    /// Assemble the decoder of the font `base_font` from its `/Encoding`, its descriptor's
    /// `/Flags` and the decoded content of its `/ToUnicode` stream.
    ///
    /// `base_table_applies` is `false` for a `Type3` font, whose encoding is its
    /// `/Differences` alone. A `/ToUnicode` that maps no code warns and is ignored, as do the
    /// unknown encoding names and symbolic fonts [`base_table`] warns about.
    pub(crate) fn new(
        encoding: Option<&EncodingSpec>,
        base_font: &Name,
        flags: Flags,
        to_unicode: Option<&[u8]>,
        base_table_applies: bool,
    ) -> Self {
        let base = if base_table_applies {
            base_table(encoding, base_font, flags)
        } else {
            None
        };
        let differences = match encoding {
            Some(EncodingSpec::Dictionary(dict)) => &dict.differences,
            Some(EncodingSpec::Named(_)) | None => &Differences::default(),
        };

        let to_unicode = to_unicode.map(ToUnicode::parse).filter(|cmap| {
            let empty = cmap.is_empty();
            if empty {
                tracing::warn!(
                    "Font {}: `/ToUnicode` maps no code: ignored",
                    String::from_utf8_lossy(base_font)
                );
            }
            !empty
        });

        Self {
            to_unicode,
            encoding: Encoding::new(base, differences),
        }
    }

    /// The text `code` stands for: its `/ToUnicode` mapping, else its glyph name's text.
    /// `None` when neither says.
    pub fn text(&self, code: Code) -> Option<Cow<'_, str>> {
        let text = self
            .to_unicode
            .as_ref()
            .and_then(|cmap| cmap.get(code))
            .map(|text| Cow::Owned(text.as_str().to_owned()))
            .or_else(|| self.glyph_name(code)?.text());
        if text.is_none() {
            tracing::debug!("code {code:?} has no text");
        }
        text
    }

    /// The glyph name the encoding gives `code`, if any.
    pub fn glyph_name(&self, code: Code) -> Option<&GlyphName> {
        self.encoding.glyph_name(code)
    }
}

/// Pop `/ToUnicode` from `dict` and return its decoded content.
///
/// A stream that cannot be resolved or decoded warns, naming `font`, and gives `None`: the
/// font still decodes through its encoding.
pub(super) fn pop_to_unicode<B>(dict: &mut RawDict<'_>, builder: &B, font: &Name) -> Option<Vec<u8>>
where
    B: Builder,
{
    match dict.pop_and_build::<Built<Stream<()>>, _>(&"ToUnicode".into(), builder) {
        Ok(stream) => stream.map(|Built(Stream { content, .. })| content),
        Err(_) => {
            tracing::warn!(
                "Font {}: `/ToUnicode` cannot be read: ignored",
                String::from_utf8_lossy(font)
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use rstest::rstest;

    use crate::content::font::tests::{bfchar, decode, font, font_with_stream};

    /// The ToUnicode CMap of `/F3` in the `encodings.pdf` fixture.
    const F3_TO_UNICODE: &[u8] = indoc! {b"
        /CIDInit /ProcSet findresource begin
        12 dict begin
        begincmap
        /CMapName /Custom def
        1 begincodespacerange
        <00> <FF>
        endcodespacerange
        1 beginbfchar
        <41> <0061>
        endbfchar
        2 beginbfrange
        <42> <43> <0062>
        <44> <45> [<0064> <00660066>]
        endbfrange
        endcmap
        CMapName currentdict /CMap defineresource pop
        end
        end
    "};

    #[rstest]
    #[case::bfchar_over_win_ansi(65, Some("a"))]
    #[case::stepped_range(67, Some("c"))]
    #[case::listed_range(69, Some("ff"))]
    #[case::falls_back_to_win_ansi(70, Some("F"))]
    fn to_unicode_wins_over_the_encoding(#[case] code: u8, #[case] expected: Option<&str>) {
        let font = font_with_stream(
            b"<</Type/Font/Subtype/Type1/BaseFont/Helvetica/FirstChar 65/LastChar 70\
              /Widths[667 667 722 722 667 611]/Encoding/WinAnsiEncoding/ToUnicode 9 0 R>>",
            F3_TO_UNICODE,
        );

        assert_eq!(decode(&font, code).as_deref(), expected);
    }

    #[rstest]
    #[case(0x41, Some("A"))]
    #[case(0x80, Some("€"))]
    #[case(0x00, None)]
    fn win_ansi_without_to_unicode(#[case] code: u8, #[case] expected: Option<&str>) {
        let font = font(
            b"<</Type/Font/Subtype/TrueType/BaseFont/Arial/FirstChar 32/Widths[278]\
              /Encoding/WinAnsiEncoding>>",
        );

        assert_eq!(decode(&font, code).as_deref(), expected);
    }

    #[rstest]
    #[case(1, Some("\u{FB01}"))]
    #[case(2, Some("Ä"))]
    #[case(3, Some("—"))]
    #[case(4, None)]
    #[case(65, Some("A"))]
    fn differences_over_standard(#[case] code: u8, #[case] expected: Option<&str>) {
        let font = font(
            b"<</Type/Font/Subtype/Type1/BaseFont/Helvetica/FirstChar 1/LastChar 3\
              /Widths[500 667 1000]/Encoding<</Differences[1 /fi /Adieresis /uni2014]>>>>",
        );

        assert_eq!(decode(&font, code).as_deref(), expected);
    }

    #[test]
    fn symbol_uses_its_built_in_table() {
        let font = font(b"<</Type/Font/Subtype/Type1/BaseFont/Symbol>>");

        assert_eq!(decode(&font, 0x61).as_deref(), Some("α"));
    }

    const WINGDINGS: &[u8] = b"<</Type/Font/Subtype/TrueType/BaseFont/Wingdings\
        /FirstChar 65/Widths[500]/FontDescriptor<</Flags 4>>";

    #[test]
    #[tracing_test::traced_test]
    fn symbolic_without_encoding_decodes_nothing() {
        let font = font(&[WINGDINGS, b">>"].concat());

        assert!(logs_contain("symbolic font Wingdings without /Encoding"));
        assert_eq!(decode(&font, 0x41), None);
    }

    #[test]
    fn symbolic_without_encoding_decodes_through_to_unicode() {
        let font = font_with_stream(
            &[WINGDINGS, b"/ToUnicode 9 0 R>>"].concat(),
            &bfchar("41", "0061"),
        );

        assert_eq!(decode(&font, 0x41).as_deref(), Some("a"));
    }

    #[test]
    #[tracing_test::traced_test]
    fn garbage_to_unicode_falls_back_to_the_encoding() {
        let font = font_with_stream(
            b"<</Type/Font/Subtype/Type1/BaseFont/Helvetica/ToUnicode 9 0 R>>",
            b"garbage",
        );

        assert!(logs_contain("`/ToUnicode` maps no code"));
        assert_eq!(decode(&font, 0x41).as_deref(), Some("A"));
    }

    #[test]
    #[tracing_test::traced_test]
    fn unresolved_to_unicode_falls_back_to_the_encoding() {
        let font = font(b"<</Type/Font/Subtype/Type1/BaseFont/Helvetica/ToUnicode 9 0 R>>");

        assert!(logs_contain("`/ToUnicode` cannot be read"));
        assert_eq!(decode(&font, 0x41).as_deref(), Some("A"));
    }
}
