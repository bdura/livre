use winnow::{
    error::{ContextError, ErrMode},
    BStr, PResult,
};

use crate::{
    extraction::{BuildFromRawDict, LiteralString, Name, OptRef, RawDict},
    Build, Builder, Extract, FromRawDict,
};

use super::{descriptor::FontDescriptor, Font};

#[derive(Debug, PartialEq, Clone)]
pub struct Widths {
    /// The first character code defined in the font’s `widths` array
    pub first_char: usize,
    /// The last character code defined in the font’s `widths` array
    pub last_char: usize,
    /// An array of `last_char - first_char + 1` numbers, each element being
    /// the glyph width for the character code that equals FirstChar plus the array index.
    /// For character codes outside the range `first_char` to `last_char`, the value of
    /// `missing_width` from the FontDescriptor entry for this font shall be used. The
    /// glyph widths shall be measured in units in which 1000 units correspond to 1 unit
    /// in text space.
    pub widths: Vec<u16>,
}

impl Widths {
    fn width(&self, cid: usize) -> Option<u16> {
        if cid < self.first_char {
            None
        } else {
            self.widths.get(cid - self.first_char).copied()
        }
    }
}

impl<'de> BuildFromRawDict<'de> for Widths {
    fn build_from_raw_dict<B>(raw_dict: &mut RawDict<'de>, builder: &B) -> PResult<Self>
    where
        B: Builder<'de>,
    {
        let first_char: usize = raw_dict.pop_and_extract_required(&"FirstChar".into())?;
        let last_char: usize = raw_dict.pop_and_extract_required(&"LastChar".into())?;
        let widths: OptRef<Vec<u16>> = raw_dict.pop_and_extract_required(&"Widths".into())?;

        let widths = widths.instantiate(builder)?;

        Ok(Self {
            first_char,
            last_char,
            widths,
        })
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct SimpleFont {
    pub base_font: Name,
    /// (Required; optional in PDF 1.0-1.7 for the standard 14 fonts; indirect reference
    /// preferred) An array of (LastChar - FirstChar + 1) numbers, each element being
    /// the glyph width for the character code that equals FirstChar plus the array index.
    /// For character codes outside the range FirstChar to LastChar, the value of
    /// MissingWidth from the FontDescriptor entry for this font shall be used. The
    /// glyph widths shall be measured in units in which 1000 units correspond to 1 unit
    /// in text space. These widths shall be consistent with the actual widths given in the
    /// font program. For more information on glyph widths and other glyph metrics, see
    /// 9.2.4, "Glyph positioning and metrics"
    pub widths: Widths,
    /// A font descriptor describing the font’s metrics other than its glyph widths
    pub font_descriptor: FontDescriptor,
    pub encoding: Option<Name>,
    //pub to_unicode: Option<ToUnicode>,
}

impl<'de> BuildFromRawDict<'de> for SimpleFont {
    fn build_from_raw_dict<B>(raw_dict: &mut RawDict<'de>, builder: &B) -> PResult<Self>
    where
        B: Builder<'de>,
    {
        let widths = Widths::build_from_raw_dict(raw_dict, builder)?;

        let font_descriptor = FontDescriptor::from_raw_dict(raw_dict)?;

        let base_font = raw_dict.pop_and_extract_required(&"BaseFont".into())?;
        let encoding = raw_dict.pop_and_extract_required(&"Encoding".into())?;

        Ok(Self {
            base_font,
            widths,
            font_descriptor,
            encoding,
        })
    }
}

impl Font for SimpleFont {
    fn ascent(&self) -> f32 {
        self.font_descriptor.ascent / 1000.0
    }

    fn descent(&self) -> f32 {
        self.font_descriptor.descent / 1000.0
    }

    fn process(&self, LiteralString(string): LiteralString) -> Vec<(char, f32, bool)> {
        string
            .iter()
            .copied()
            .map(|c| (c as char, self.width(c), c == b' '))
            .collect()
    }

    /// Compute the font's width.
    ///
    /// In the case of simple fonts, the width is explicitly described within a the font
    /// dictionnary's `Width` key (see [`Widths`] for more context).
    ///
    /// In case the width is not explicitly defined through the [`Widths`] object, then the
    /// processor should use the [`FontDescriptor`]'s `missing_width` parameter.
    fn width(&self, code: u8) -> f32 {
        self.widths
            .width(code as usize)
            .map(f32::from)
            .unwrap_or(self.font_descriptor.missing_width)
            / 1000.0
    }

    fn name(&self) -> &str {
        let name = &self.font_descriptor.font_name;
        std::str::from_utf8(name).expect("fonts have UTF-8-encoded names.")
    }
}
