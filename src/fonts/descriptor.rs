use livre_derive::{BuildFromRawDict, FromRawDict};
use winnow::{BStr, PResult};

use crate::{
    extraction::{extract, Extract, Name, Rectangle},
    follow_refs::{Build, Builder},
};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct FontFlags {
    /// All glyphs have the same width (as opposed to proportional or
    /// variable-pitch fonts, which have different widths).
    pub fixed_pitch: bool,
    /// Glyphs have serifs, which are short strokes drawn at an angle on the
    /// top and bottom of glyph stems. (Sans serif fonts do not have serifs.)
    pub serif: bool,
    /// Font contains glyphs outside the Standard Latin character set. This
    /// flag and the Nonsymbolic flag shall not both be set or both be clear.
    pub symbolic: bool,
    /// Glyphs resemble cursive handwriting.
    pub script: bool,
    /// Font uses the Standard Latin character set or a subset of it. This flag
    /// and the Symbolic flag shall not both be set or both be clear.
    pub nonsymbolic: bool,
    /// Glyphs have dominant vertical strokes that are slanted.
    pub italic: bool,
    /// Font contains no lowercase letters; typically used for display
    /// purposes, such as for titles or headlines.
    pub all_cap: bool,
    /// Font contains both uppercase and lowercase letters. The uppercase
    /// letters are similar to those in the regular version of the same typeface
    /// family. The glyphs for the lowercase letters have the same shapes as
    /// the corresponding uppercase letters, but they are sized and their
    /// proportions adjusted so that they have the same size and stroke
    /// weight as lowercase glyphs in the same typeface family.
    pub small_cap: bool,
    /// The ForceBold flag (bit 19) shall determine whether bold glyphs shall be painted with extra pixels even
    /// at very small text sizes by a PDF processor. If the ForceBold flag is set, features of bold glyphs may be
    /// thickened at small text sizes.
    pub force_bold: bool,
}

impl From<u32> for FontFlags {
    fn from(num: u32) -> Self {
        let fixed_pitch = num & 1 == 1;
        let serif = (num >> 1) & 1 == 1;
        let symbolic = (num >> 2) & 1 == 1;
        let script = (num >> 3) & 1 == 1;
        let nonsymbolic = (num >> 5) & 1 == 1;
        let italic = (num >> 6) & 1 == 1;
        let all_cap = (num >> 16) & 1 == 1;
        let small_cap = (num >> 17) & 1 == 1;
        let force_bold = (num >> 18) & 1 == 1;

        Self {
            fixed_pitch,
            serif,
            symbolic,
            script,
            nonsymbolic,
            italic,
            all_cap,
            small_cap,
            force_bold,
        }
    }
}

impl Extract<'_> for FontFlags {
    fn extract(input: &mut &'_ winnow::BStr) -> winnow::PResult<Self> {
        let flags: u32 = extract(input)?;
        Ok(flags.into())
    }
}

impl Build for FontFlags {
    fn build<B>(input: &mut &BStr, _: &B) -> PResult<Self>
    where
        B: Builder,
    {
        extract(input)
    }
}

/// Describes the overall properties of a font.
#[derive(Debug, PartialEq, Clone, FromRawDict, BuildFromRawDict)]
pub struct FontDescriptor {
    /// The PostScript name of the font.
    pub font_name: Name,
    /// A byte string specifying the preferred font family name.
    ///
    /// Example from the specification:
    ///
    /// > For the font Times Bold Italic, the `FontFamily` is "Times".
    pub font_family: Option<FontStretch>,
    pub font_stretch: Option<Name>,
    /// The weight (thickness) of the font.
    pub font_weight: Option<FontWeight>,
    /// A collection of flags defining various characteristics of the font.
    pub flags: FontFlags,
    /// A rectangle representing the minimal bounding box that would contain every glyph.
    #[livre(rename = "FontBBox")]
    pub font_bbox: Rectangle,
    /// The angle between the vertical and the font's dominant vertical strokes.
    /// Express in degrees, counter-clockwise angles being positive.
    pub italic_angle: f32,
    /// Maximum height above the baseline reached by the glyphs.
    ///
    /// NOTE: optional for type 3 fonts.
    pub ascent: f32,
    /// Maximum depth below the baseline reached by the glyphs. Must be negative.
    ///
    /// NOTE: optional for type 3 fonts.
    pub descent: f32,
    /// Spacing between two consecutive baselines of text.
    #[livre(default = 0.0)]
    pub leading: f32,
    /// ///
    /// pub cap_height: f32,
    /// pub x_height: Option<f32>,
    /// pub stem_v: f32,
    /// #[livre(default)]
    /// pub stem_h: f32,
    /// The average width of glyphs in the font.
    #[livre(default = 0.0)]
    pub avg_width: f32,
    /// The maximum width of glyphs in the font.
    #[livre(default = 0.0)]
    pub max_width: f32,
    /// The maximum width of glyphs in the font.
    #[livre(default)]
    pub missing_width: f32,
    // font_file
    // font_file2
    // font_file3
    // char_set
}

/// The weight (thickness) of the font. The interpretation varies from font to font.
///
/// According to the PDF specification:
///
/// > The value "shall be one of 100, 200, 300, 400, 500, 600, 700, 800, or 900,
/// > where each number indicates a weight that is at least as dark as its predecessor.
/// > A value of 400 shall indicate a normal weight; 700 shall indicate bold.
///
/// The specification also notes that the definition of `FontWeight` in PDF matches
/// the CSS font-weight property, but may be more constrained than font weights
/// used by various font formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontWeight {
    W100,
    W200,
    W300,
    W400,
    W500,
    W600,
    W700,
    W800,
    W900,
}

impl Extract<'_> for FontWeight {
    fn extract(input: &mut &'_ BStr) -> PResult<Self> {
        let weight: u16 = extract(input)?;

        let inner = match weight {
            100 => Self::W100,
            200 => Self::W200,
            300 => Self::W300,
            400 => Self::W400,
            500 => Self::W500,
            600 => Self::W600,
            700 => Self::W700,
            800 => Self::W800,
            900 => Self::W900,
            _ => unreachable!("the `{weight}` modality is not listed in the PDF specification"),
        };

        Ok(inner)
    }
}

impl Build for FontWeight {
    fn build<B>(input: &mut &BStr, _: &B) -> PResult<Self>
    where
        B: Builder,
    {
        extract(input)
    }
}

/// The font stretch value. The interpretation varies from font to font.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontStretch {
    UltraCondensed,
    ExtraCondensed,
    Condensed,
    SemiCondensed,
    Normal,
    SemiExpanded,
    Expanded,
    ExtraExpanded,
    UltraExpanded,
}

impl Extract<'_> for FontStretch {
    fn extract(input: &mut &BStr) -> PResult<Self> {
        let name: Name = extract(input)?;
        let inner = match name.as_slice() {
            b"UltraCondensed" => Self::UltraCondensed,
            b"ExtraCondensed" => Self::ExtraCondensed,
            b"Condensed" => Self::Condensed,
            b"SemiCondensed" => Self::SemiCondensed,
            b"Normal" => Self::Normal,
            b"SemiExpanded" => Self::SemiExpanded,
            b"Expanded" => Self::Expanded,
            b"ExtraExpanded" => Self::ExtraExpanded,
            b"UltraExpanded" => Self::UltraExpanded,
            _ => unreachable!(
                "the `{:?}` modality is not listed in the PDF specification",
                name
            ),
        };

        Ok(inner)
    }
}

impl Build for FontStretch {
    fn build<B>(input: &mut &BStr, _: &B) -> PResult<Self>
    where
        B: Builder,
    {
        extract(input)
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use indoc::indoc;
    use rstest::rstest;

    use super::*;

    impl FontFlags {
        fn from_bytes_repr(flags: &[u8; 9]) -> Self {
            Self {
                fixed_pitch: flags[0] == b'1',
                serif: flags[1] == b'1',
                symbolic: flags[2] == b'1',
                script: flags[3] == b'1',
                nonsymbolic: flags[4] == b'1',
                italic: flags[5] == b'1',
                all_cap: flags[6] == b'1',
                small_cap: flags[7] == b'1',
                force_bold: flags[8] == b'1',
            }
        }
    }

    #[rstest]
    #[case(b"0", FontFlags::from_bytes_repr(b"000000000"))]
    #[case(b"262178", FontFlags::from_bytes_repr(b"010010001"))]
    #[case(b"1", FontFlags::from_bytes_repr(b"100000000"))]
    #[case(b"3", FontFlags::from_bytes_repr(b"110000000"))]
    #[case(b"2", FontFlags::from_bytes_repr(b"010000000"))]
    #[case(b"4294967295", FontFlags::from_bytes_repr(b"111111111"))]
    #[case(b"/UltraCondensed", FontStretch::UltraCondensed)]
    #[case(b"/Normal", FontStretch::Normal)]
    #[case(b"100", FontWeight::W100)]
    #[case(
        indoc! {b"
            <</Type /FontDescriptor
            /FontName /AGaramond-Semibold
            /Flags 262178
            /FontBBox [-177 -269 1123 866]
            /MissingWidth 255
            /StemV 105
            /StemH 45
            /CapHeight 660
            /XHeight 394
            /Ascent 720
            /Descent -270
            /Leading 83
            /MaxWidth 1212
            /AvgWidth 478
            /ItalicAngle 0
            >>
        "},
        FontDescriptor {
            font_name: "AGaramond-Semibold".into(),
            font_family: None,
            font_stretch: None,
            font_weight: None,
            flags: FontFlags::from_bytes_repr(b"010010001"),
            font_bbox: Rectangle::from((-177.0, -269.0, 1123.0, 866.0)),
            italic_angle: 0.0,
            ascent: 720.0,
            descent: -270.0,
            leading: 83.0,
            // cap_height: 660.0,
            // x_height: Some(394.0),
            // stem_v: 105.0,
            // stem_h: 45.0,
            avg_width: 478.0,
            max_width: 1212.0,
            missing_width: 255.0,
        }
    )]
    fn extraction<'de, T>(#[case] input: &'de [u8], #[case] expected: T)
    where
        T: Extract<'de> + Debug + PartialEq,
    {
        let result = extract(&mut input.as_ref()).unwrap();
        assert_eq!(expected, result)
    }
}
