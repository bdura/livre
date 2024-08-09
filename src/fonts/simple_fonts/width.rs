use serde::Deserialize;

use crate::{parsers::OptRef, structure::Build};

#[derive(Debug, PartialEq, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct WidthsTransient {
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
    pub widths: OptRef<Vec<u16>>,
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
pub struct Widths {
    pub first_char: usize,
    pub last_char: usize,
    pub widths: Vec<u16>,
}

impl Build for WidthsTransient {
    type Output = Widths;

    fn build(self, doc: &crate::structure::Document) -> Self::Output {
        let Self {
            first_char,
            last_char,
            widths,
        } = self;
        let widths = widths.build(doc);
        Widths {
            first_char,
            last_char,
            widths,
        }
    }
}

impl Widths {
    pub fn width(&self, cid: usize) -> Option<u16> {
        if cid < self.first_char {
            None
        } else {
            self.widths.get(cid - self.first_char).copied()
        }
    }
}
