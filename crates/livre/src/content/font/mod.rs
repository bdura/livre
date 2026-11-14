//! Font dictionaries, read as far as positioning text needs: how a shown string splits into
//! character codes, and how wide each code's glyph is (ISO 32000-2, 9.5 to 9.7).
//!
//! A [`Font`] is told apart by its `/Subtype`. Simple fonts (`Type1`, `MMType1`, `TrueType`)
//! and `Type3` fonts read one byte per code and carry a [`Widths`] table; `Type0` fonts are a
//! placeholder that reads two bytes per code and reports zero widths.

mod code;
mod composite;
mod simple;
mod type3;
mod widths;

use std::{collections::HashMap, fmt};

use winnow::{
    error::{ContextError, ErrMode, FromExternalError, StrContext},
    BStr, ModalResult,
};

pub use code::{Code, Codes};
pub use composite::CompositeFont;
pub use simple::{FontDescriptor, SimpleFont};
pub use type3::Type3Font;
pub use widths::{GlyphWidth, Widths};

use crate::{
    content::{Glyph, Matrix, Text},
    extraction::{FromRawDict, Name, RawDict},
    follow_refs::{BuildFromRawDict, Builder},
};

/// A font dictionary, by `/Subtype`.
#[derive(Debug, Clone, PartialEq)]
pub enum Font {
    /// `Type1`, `MMType1` or `TrueType`.
    Simple(SimpleFont),
    /// `Type3`.
    Type3(Type3Font),
    /// `Type0`, as a placeholder.
    Composite(CompositeFont),
}

/// The fonts a page's content stream can select with `Tf`, by resource name: the page's
/// `/Resources /Font` map with every reference resolved.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Fonts(HashMap<Name, Font>);

impl Fonts {
    /// No fonts: a page whose resources have no `/Font` entry.
    pub fn empty() -> Self {
        Self::default()
    }

    /// The font under the resource name `name`, as `Tf` spells it.
    pub fn get(&self, name: &Name) -> Option<&Font> {
        self.0.get(name)
    }
}

impl FromIterator<(Name, Font)> for Fonts {
    fn from_iter<I: IntoIterator<Item = (Name, Font)>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

/// The text space length of a thousandth: glyph space for `Type1`, `TrueType` and, until
/// their widths are read, composite fonts (ISO 32000-2, 9.2.4).
const THOUSANDTH: Matrix<Glyph, Text> = Matrix::new(0.001, 0.0, 0.0, 0.001, 0.0, 0.0);

impl Font {
    /// The matrix from the font's glyph space to text space: a scale by 1/1000, or the
    /// `/FontMatrix` of a `Type3` font.
    pub fn glyph_to_text(&self) -> Matrix<Glyph, Text> {
        match self {
            Font::Simple(_) | Font::Composite(_) => THOUSANDTH,
            Font::Type3(font) => font.font_matrix,
        }
    }

    /// The width of `code`'s glyph. Always zero for a composite font.
    pub fn width(&self, code: Code) -> GlyphWidth {
        match self {
            Font::Simple(font) => font.widths.width(code),
            Font::Type3(font) => font.widths.width(code),
            Font::Composite(_) => GlyphWidth(0.0),
        }
    }
}

/// The cause attached to the error of a font with an unsupported `/Subtype`.
#[derive(Debug)]
struct UnknownSubtype(Name);

impl fmt::Display for UnknownSubtype {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown font subtype /{}",
            String::from_utf8_lossy(&self.0)
        )
    }
}

impl std::error::Error for UnknownSubtype {}

/// Dispatches on `/Subtype` to the variant's reader.
///
/// A missing `/Subtype` and an unknown one are backtrack errors labelled `/Subtype`; the
/// latter's cause names the subtype.
impl BuildFromRawDict for Font {
    fn build_from_raw_dict<B>(dict: &mut RawDict<'_>, builder: &B) -> ModalResult<Self>
    where
        B: Builder,
    {
        let label = StrContext::Label("/Subtype");
        let subtype: Name = dict
            .pop_and_extract(&"Subtype".into())
            .unwrap_or_else(|| Err(ErrMode::Backtrack(ContextError::new())))
            .map_err(|e| {
                e.map(|mut e| {
                    e.push(label.clone());
                    e
                })
            })?;

        match subtype.0.as_slice() {
            b"Type1" | b"MMType1" | b"TrueType" => {
                SimpleFont::build_from_raw_dict(dict, builder).map(Font::Simple)
            }
            b"Type3" => Type3Font::build_from_raw_dict(dict, builder).map(Font::Type3),
            b"Type0" => CompositeFont::build_from_raw_dict(dict, builder).map(Font::Composite),
            _ => {
                // The input is only a type witness: `ContextError` does not record it.
                let input: &BStr = b"".as_slice().as_ref();
                let mut e = ContextError::from_external_error(&input, UnknownSubtype(subtype));
                e.push(label);
                Err(ErrMode::Backtrack(e))
            }
        }
    }
}

/// Reads the font without following references: an indirect `/FontDescriptor` or `/Widths`
/// is an error here. Build through a [`Builder`] to follow them.
impl FromRawDict<'_> for Font {
    fn from_raw_dict(dict: &mut RawDict<'_>) -> ModalResult<Self> {
        Self::build_from_raw_dict(dict, &())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::{extraction::ReferenceId, follow_refs::Build};

    /// The font in the dictionary literal `input`, with no reference to follow.
    pub(crate) fn font(input: &[u8]) -> Font {
        font_with(input, &HashMap::new())
    }

    /// The font in the dictionary literal `input`, following references through `objects`.
    pub(crate) fn font_with(input: &[u8], objects: &HashMap<ReferenceId, &BStr>) -> Font {
        Font::build(&mut input.as_ref(), objects).unwrap()
    }

    /// A standard-14 font dictionary with a `/Widths` table, so parsing it does not warn.
    pub(crate) const HELVETICA: &[u8] =
        b"<</Type/Font/Subtype/Type1/BaseFont/Helvetica/FirstChar 32/LastChar 32/Widths[278]>>";

    /// The fonts of a page, each given by its resource name and its dictionary literal.
    pub(crate) fn fonts(entries: &[(&str, &[u8])]) -> Fonts {
        entries
            .iter()
            .map(|&(name, input)| (Name::from(name), font(input)))
            .collect()
    }

    #[test]
    fn fonts_get_finds_by_resource_name() {
        let fonts = fonts(&[("F1", HELVETICA)]);
        assert!(fonts.get(&"F1".into()).is_some());
        assert!(fonts.get(&"F2".into()).is_none());
        assert!(Fonts::empty().get(&"F1".into()).is_none());
    }

    #[test]
    fn unknown_subtype_is_named() {
        let error = Font::build(&mut b"<</Type/Font/Subtype/Foo>>".as_slice().as_ref(), &())
            .unwrap_err()
            .into_inner()
            .unwrap();

        let message = error.to_string();
        assert!(message.contains("/Subtype"), "{message}");
        assert!(message.contains("Foo"), "{message}");
    }

    #[test]
    fn missing_subtype_is_labelled() {
        let error = Font::build(&mut b"<</Type/Font>>".as_slice().as_ref(), &())
            .unwrap_err()
            .into_inner()
            .unwrap();

        assert!(error.to_string().contains("/Subtype"), "{error}");
    }

    #[test]
    fn extracts_without_builder() {
        let font: Font = crate::extraction::extract(
            &mut b"<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>"
                .as_slice()
                .as_ref(),
        )
        .unwrap();
        assert!(matches!(font, Font::Simple(_)));
    }
}
