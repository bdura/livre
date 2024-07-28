use livre_data::Rectangle;
use serde::Deserialize;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize)]
#[serde(from="u32")]
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
        let serif = num >> 1 & 1 == 1;
        let symbolic = num >> 2 & 1 == 1;
        let script = num >> 3 & 1 == 1;
        let nonsymbolic = num >> 5 & 1 == 1;
        let italic = num >> 6 & 1 == 1;
        let all_cap = num >> 16 & 1 == 1;
        let small_cap = num >> 17 & 1 == 1;
        let force_bold = num >> 18 & 1 == 1;

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

#[derive(Debug, PartialEq, Clone, Deserialize)]
#[serde(rename_all="PascalCase")]
pub struct FontDescriptor {
    pub font_name: String,
    pub font_family: Option<String>,
    pub font_stretch: Option<String>,
    pub font_weight: Option<u16>,
    pub flags: FontFlags,
    #[serde(rename = "FontBBox")]
    pub font_bbox: Rectangle,
    pub italic_angle: f32,
    pub ascent: f32,
    pub descent: f32,
    #[serde(default)]
    pub leading: f32,
    pub cap_height: f32,
    pub x_height: Option<f32>,
    pub stem_v: f32,
    pub stem_h: f32,
    #[serde(default)]
    pub avg_width: f32,
    #[serde(default)]
    pub max_width: f32,
    #[serde(default)]
    pub missing_width: f32,
    // font_file
    // font_file2
    // font_file3
    // char_set
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use livre_serde::extract_deserialize;
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
    fn font_flags(#[case] input: &[u8], #[case] expected: FontFlags) {
        let (_, flags) = extract_deserialize(input).unwrap();
        assert_eq!(expected, flags)
    }

    #[rstest]
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
            font_name: String::from("AGaramond-Semibold"),
            font_family: None,
            font_stretch: None,
            font_weight: None,
            flags: FontFlags::from_bytes_repr(b"010010001"),
            font_bbox: Rectangle::from_ll_ur(-177.0, -269.0, 1123.0, 866.0),
            italic_angle: 0.0,
            ascent: 720.0,
            descent: -270.0,
            leading: 83.0,
            cap_height: 660.0,
            x_height: Some(394.0),
            stem_v: 105.0,
            stem_h: 45.0,
            avg_width: 478.0,
            max_width: 1212.0,
            missing_width: 255.0,
        }
    )]
    fn font_descriptor(#[case] input: &[u8], #[case] expected: FontDescriptor) {
        let (_, fd) = extract_deserialize(input).unwrap();
        assert_eq!(expected, fd);
    }
}
