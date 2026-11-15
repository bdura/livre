//! `Type0` composite fonts (ISO 32000-2, 9.7): an encoding CMap from codes to CIDs, and a
//! descendant CIDFont whose metrics are keyed by them.

use winnow::{
    combinator::{alt, opt, trace},
    error::{ContextError, ErrMode, StrContext},
    BStr, ModalResult, Parser,
};

use super::{decoder::pop_to_unicode, Cid, CidFont, Code, EncodingCMap, ToUnicode};
use crate::{
    extraction::{extract, Extract, Name, RawDict, Reference, Stream},
    follow_refs::{Build, BuildFromRawDict, Builder, Built},
};

/// A `Type0` font (ISO 32000-2, 9.7): a CMap from codes to CIDs, and a CIDFont keyed by them.
#[derive(Debug, Clone, PartialEq)]
pub struct CompositeFont {
    /// The font's name, `/BaseFont`.
    pub base_font: Name,
    /// `/Encoding`: how strings split into codes, and which CID each code selects.
    pub encoding: EncodingCMap,
    /// The first entry of `/DescendantFonts`, which holds the CID-keyed metrics.
    pub descendant: CidFont,
    /// `/ToUnicode`: the only route from a code to text here; the character collection
    /// route through `/CIDSystemInfo` is not taken.
    pub to_unicode: Option<ToUnicode>,
}

impl CompositeFont {
    /// The CID `code` selects: the encoding's, else CID 0, the `.notdef` of every CIDFont
    /// (ISO 32000-2, 9.7.6.3).
    pub fn cid(&self, code: Code) -> Cid {
        self.encoding.cid(code).unwrap_or(Cid(0))
    }
}

/// The `/Encoding` of a `Type0` font: a predefined CMap name or an embedded CMap stream,
/// either possibly behind a reference.
#[derive(Debug)]
enum NameOrStream {
    Name(Name),
    Stream(Stream<CMapStreamDict>),
}

/// The entries of a CMap stream dictionary this crate reads.
#[derive(Debug, BuildFromRawDict)]
struct CMapStreamDict {
    /// `/WMode`, the fallback when the CMap body declares none.
    w_mode: Option<u8>,
}

impl Build for NameOrStream {
    fn build<B>(input: &mut &BStr, builder: &B) -> ModalResult<Self>
    where
        B: Builder,
    {
        if let Some(reference) = opt(Reference::<Self>::extract).parse_next(input)? {
            return builder.build_reference(reference);
        }
        trace(
            "livre-name-or-stream",
            alt((extract.map(Self::Name), |i: &mut &BStr| {
                Stream::<CMapStreamDict>::build(i, builder).map(Self::Stream)
            })),
        )
        .parse_next(input)
    }
}

/// The fields read by derive; `/ToUnicode` is popped separately so that an unreadable one
/// only warns.
#[derive(BuildFromRawDict)]
struct CompositeFontDict {
    base_font: Name,
    encoding: NameOrStream,
    descendant_fonts: Built<Vec<CidFont>>,
}

/// Reads `/BaseFont`, `/Encoding`, `/DescendantFonts` and `/ToUnicode`.
///
/// An empty `/DescendantFonts` is a backtrack error labelled `/DescendantFonts`; more than one
/// entry warns and the first is used. A predefined `/Encoding` other than the identities
/// warns and is read as `Identity-H`. An unreadable or empty `/ToUnicode` warns and is
/// ignored.
impl BuildFromRawDict for CompositeFont {
    fn build_from_raw_dict<B>(dict: &mut RawDict<'_>, builder: &B) -> ModalResult<Self>
    where
        B: Builder,
    {
        let CompositeFontDict {
            base_font,
            encoding,
            descendant_fonts: Built(descendant_fonts),
        } = CompositeFontDict::build_from_raw_dict(dict, builder)?;
        let font_name = String::from_utf8_lossy(&base_font).into_owned();

        if descendant_fonts.len() > 1 {
            tracing::warn!(
                "Font {font_name}: `/DescendantFonts` has {} entries: only the first is used",
                descendant_fonts.len()
            );
        }
        let Some(descendant) = descendant_fonts.into_iter().next() else {
            let mut e = ContextError::new();
            e.push(StrContext::Label("/DescendantFonts"));
            return Err(ErrMode::Backtrack(e));
        };

        let encoding = match encoding {
            NameOrStream::Name(name) => {
                // `from_name` names the CMap in its warning; the span names the font.
                let _font = tracing::warn_span!("font", name = %font_name).entered();
                EncodingCMap::from_name(&name)
            }
            NameOrStream::Stream(Stream {
                structured: CMapStreamDict { w_mode },
                content,
            }) => EncodingCMap::from_stream(&content, w_mode),
        };

        let to_unicode = pop_to_unicode(dict, builder, &base_font)
            .map(|content| ToUnicode::parse(&content))
            .filter(|cmap| {
                let empty = cmap.is_empty();
                if empty {
                    tracing::warn!("Font {font_name}: `/ToUnicode` maps no code: ignored");
                }
                !empty
            });

        Ok(Self {
            base_font,
            encoding,
            descendant,
            to_unicode,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use winnow::BStr;

    use crate::{
        content::font::{tests::font_with, Code, Font, GlyphWidth, WritingMode},
        extraction::{PDFString, ReferenceId},
        follow_refs::Build,
    };

    /// The indirect object `id 0 obj` holding `body`.
    fn object(id: usize, body: &[u8]) -> (usize, Vec<u8>) {
        let object = [format!("{id} 0 obj\n").as_bytes(), body, b"\nendobj"].concat();
        (id, object)
    }

    /// The indirect object `id 0 obj` holding a stream of `content`, its dictionary entries
    /// `entries` written before `/Length`.
    fn stream(id: usize, entries: &str, content: &[u8]) -> (usize, Vec<u8>) {
        let body = [
            format!("<<{entries}/Length {}>>\nstream\n", content.len()).as_bytes(),
            content,
            b"\nendstream",
        ]
        .concat();
        object(id, &body)
    }

    /// The font in `input`, its references resolved through `objects`.
    fn build(input: &[u8], objects: &[(usize, Vec<u8>)]) -> Font {
        let objects: HashMap<ReferenceId, &BStr> = objects
            .iter()
            .map(|(id, object)| ((*id, 0).into(), object.as_slice().as_ref()))
            .collect();
        font_with(input, &objects)
    }

    /// A ToUnicode CMap over `codespace` with the `bfchar` and `bfrange` bodies given.
    fn to_unicode(codespace: &str, bfchar: &[&str], bfrange: &[&str]) -> Vec<u8> {
        let mut body = format!(
            "begincmap\n{} begincodespacerange\n{codespace}\nendcodespacerange\n",
            codespace.lines().count()
        );
        if !bfchar.is_empty() {
            body += &format!(
                "{} beginbfchar\n{}\nendbfchar\n",
                bfchar.len(),
                bfchar.join("\n")
            );
        }
        if !bfrange.is_empty() {
            body += &format!(
                "{} beginbfrange\n{}\nendbfrange\n",
                bfrange.len(),
                bfrange.join("\n")
            );
        }
        body += "endcmap";
        body.into_bytes()
    }

    fn string(bytes: &[u8]) -> PDFString {
        PDFString(bytes.to_vec())
    }

    fn text(font: &Font, code: Code) -> Option<String> {
        font.decode(code).map(Into::into)
    }

    /// `/F1` of `composite.pdf`: `Identity-H`, `/DW 500 /W [1 [722 667] 3 5 600]`.
    fn f1() -> Font {
        build(
            b"<</Type/Font/Subtype/Type0/BaseFont/F1/Encoding/Identity-H/DescendantFonts[6 0 R]/ToUnicode 8 0 R>>",
            &[
                object(
                    6,
                    b"<</Type/Font/Subtype/CIDFontType2/BaseFont/F1/DW 500/W[1[722 667]3 5 600]>>",
                ),
                stream(
                    8,
                    "",
                    &to_unicode(
                        "<0000> <FFFF>",
                        &["<0001> <0041>"],
                        &["<0002> <0005> <0042>"],
                    ),
                ),
            ],
        )
    }

    /// The embedded CMap of `/F2`: one-byte codes up to `80`, two-byte from `8140`.
    const MIXED: &[u8] = b"%!PS-Adobe-3.0 Resource-CMap
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CMapName /Mixed def
/WMode 0 def
2 begincodespacerange
<00> <80>
<8140> <FFFF>
endcodespacerange
1 begincidrange
<20> <7E> 1
endcidrange
1 begincidchar
<8140> 200
endcidchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end";

    /// `/F2` of `composite.pdf`: the [`MIXED`] CMap, `/DW 1000 /W [34 [667] 200 [1000]]`.
    fn f2() -> Font {
        build(
            b"<</Type/Font/Subtype/Type0/BaseFont/F2/Encoding 7 0 R/DescendantFonts[6 0 R]/ToUnicode 8 0 R>>",
            &[
                object(
                    6,
                    b"<</Type/Font/Subtype/CIDFontType0/BaseFont/F2/DW 1000/W[34[667]200[1000]]>>",
                ),
                stream(7, "/Type/CMap/CMapName/Mixed", MIXED),
                stream(
                    8,
                    "",
                    &to_unicode(
                        "<00> <80>\n<8140> <FFFF>",
                        &["<20> <0020>", "<41> <0041>", "<8140> <3042>"],
                        &[],
                    ),
                ),
            ],
        )
    }

    /// `/F3` of `composite.pdf`: `Identity-V`, `/W2 [1 [-500 250 880]]`, default `/DW2`.
    fn f3() -> Font {
        build(
            b"<</Type/Font/Subtype/Type0/BaseFont/F3/Encoding/Identity-V/DescendantFonts[6 0 R]>>",
            &[object(
                6,
                b"<</Type/Font/Subtype/CIDFontType2/BaseFont/F3/W2[1[-500 250 880]]>>",
            )],
        )
    }

    fn assert_displacement(font: &Font, code: Code, expected: (f32, f32)) {
        let displacement = font.glyph_displacement(code);
        let actual = (displacement.dx, displacement.dy);
        assert!(
            (actual.0 - expected.0).abs() < 1e-6 && (actual.1 - expected.1).abs() < 1e-6,
            "{actual:?} != {expected:?}"
        );
    }

    #[test]
    fn identity_h_font() {
        let font = f1();

        assert!(matches!(font, Font::Composite(_)), "{font:?}");
        let codes: Vec<_> = font
            .codes(&string(&[0x00, 0x01, 0x00, 0x02, 0x00, 0x06]))
            .collect();
        assert_eq!(
            codes,
            [Code::two_bytes(1), Code::two_bytes(2), Code::two_bytes(6)]
        );
        assert_eq!(font.width(Code::two_bytes(1)), GlyphWidth(722.0));
        assert_eq!(font.width(Code::two_bytes(6)), GlyphWidth(500.0));
        assert_eq!(text(&font, Code::two_bytes(1)).as_deref(), Some("A"));
        assert_eq!(text(&font, Code::two_bytes(5)).as_deref(), Some("E"));
        assert_eq!(text(&font, Code::two_bytes(6)), None);
        assert_eq!(font.writing_mode(), WritingMode::Horizontal);
        assert_displacement(&font, Code::two_bytes(1), (0.722, 0.0));
    }

    #[test]
    fn embedded_mixed_cmap() {
        let font = f2();

        let codes: Vec<_> = font.codes(&string(&[0x41, 0x81, 0x40, 0x41])).collect();
        assert_eq!(
            codes.iter().map(Code::byte_count).collect::<Vec<_>>(),
            [1, 2, 1]
        );
        // CID 34.
        assert_eq!(font.width(Code::one_byte(0x41)), GlyphWidth(667.0));
        // CID 200.
        assert_eq!(font.width(Code::two_bytes(0x8140)), GlyphWidth(1000.0));
        // CID 1, through `/DW`.
        assert_eq!(font.width(Code::one_byte(0x20)), GlyphWidth(1000.0));
        assert_eq!(text(&font, Code::two_bytes(0x8140)).as_deref(), Some("あ"));
        assert_eq!(font.writing_mode(), WritingMode::Horizontal);
    }

    #[test]
    fn identity_v_font() {
        let font = f3();

        assert_eq!(font.writing_mode(), WritingMode::Vertical);
        assert_displacement(&font, Code::two_bytes(1), (0.0, -0.5));
        assert_displacement(&font, Code::two_bytes(2), (0.0, -1.0));
    }

    #[test]
    #[tracing_test::traced_test]
    fn predefined_cjk_cmap_is_read_as_identity() {
        let font = build(
            b"<</Type/Font/Subtype/Type0/BaseFont/SimSun/Encoding/UniGB-UCS2-H/DescendantFonts[6 0 R]>>",
            &[object(
                6,
                b"<</Type/Font/Subtype/CIDFontType2/BaseFont/SimSun/W[16706[333]]>>",
            )],
        );

        assert!(logs_contain(
            "predefined CMap /UniGB-UCS2-H not shipped: treated as Identity-H"
        ));
        let codes: Vec<_> = font.codes(&string(&[0x41, 0x42])).collect();
        assert_eq!(codes, [Code::two_bytes(0x4142)]);
        // CID 0x4142 = 16706.
        assert_eq!(font.width(Code::two_bytes(0x4142)), GlyphWidth(333.0));
    }

    #[test]
    fn empty_descendant_fonts_is_an_error() {
        let error = Font::build(
            &mut b"<</Type/Font/Subtype/Type0/BaseFont/F/Encoding/Identity-H/DescendantFonts[]>>"
                .as_slice()
                .as_ref(),
            &(),
        )
        .unwrap_err()
        .into_inner()
        .unwrap();

        assert!(error.to_string().contains("/DescendantFonts"), "{error}");
    }

    #[test]
    #[tracing_test::traced_test]
    fn several_descendant_fonts_take_the_first() {
        let font = build(
            b"<</Type/Font/Subtype/Type0/BaseFont/F/Encoding/Identity-H/DescendantFonts[6 0 R 7 0 R]>>",
            &[
                object(6, b"<</Type/Font/Subtype/CIDFontType2/BaseFont/F/DW 600>>"),
                object(7, b"<</Type/Font/Subtype/CIDFontType2/BaseFont/G/DW 700>>"),
            ],
        );

        assert!(logs_contain("only the first is used"));
        assert_eq!(font.width(Code::two_bytes(1)), GlyphWidth(600.0));
    }

    /// An `Identity-H` font named Calibri whose `A` (CID 0x41) is 500 wide and `B` 600.
    fn calibri() -> Font {
        build(
            b"<</Type/Font/Subtype/Type0/BaseFont/ABCDEF+Calibri/DescendantFonts[6 0 R]/Encoding/Identity-H>>",
            &[object(
                6,
                b"<</Type/Font/Subtype/CIDFontType2/BaseFont/ABCDEF+Calibri/W[65[500 600]]>>",
            )],
        )
    }

    #[test]
    fn composite_widths_come_from_w() {
        let font = calibri();

        assert!(matches!(font, Font::Composite(_)), "{font:?}");
        assert_eq!(font.width(Code::two_bytes(0x41)), GlyphWidth(500.0));
        assert_eq!(font.width(Code::two_bytes(0x42)), GlyphWidth(600.0));
        // Past `/W`: the default `/DW` of 1000.
        assert_eq!(font.width(Code::two_bytes(0)), GlyphWidth(1000.0));
        // A one-byte code has no CID under `Identity-H`: CID 0, `/DW`.
        assert_eq!(font.width(Code::one_byte(32)), GlyphWidth(1000.0));
    }

    #[test]
    fn two_bytes_per_code() {
        let codes: Vec<_> = calibri().codes(&string(b"\x00\x41\x00\x42")).collect();

        assert_eq!(codes, [Code::two_bytes(0x41), Code::two_bytes(0x42)]);
    }

    #[test]
    #[tracing_test::traced_test]
    fn odd_trailing_byte_is_dropped() {
        let codes: Vec<_> = calibri().codes(&string(b"\x00\x41\x00")).collect();

        assert_eq!(codes, [Code::two_bytes(0x41)]);
        assert!(logs_contain("truncated code"));
    }
}
