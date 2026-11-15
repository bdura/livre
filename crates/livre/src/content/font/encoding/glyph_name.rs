//! Glyph names and the text they stand for, by the Adobe Glyph List and its specification's
//! rules for names outside the list.

use std::borrow::Cow;

use livre_font_data::glyph_list;

use crate::extraction::Name;

/// A glyph name, from a built-in table or a `/Differences` array.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlyphName(pub(super) Cow<'static, str>);

impl GlyphName {
    /// The text the name stands for: the glyph list, then the AGL rules (ISO 32000-2, 9.10.2).
    ///
    /// A name in the list is looked up whole. Otherwise everything from the first `.` is
    /// dropped, and each `_`-separated component must be in the list, or be `uniXXXX` (one or
    /// more groups of four uppercase hex digits, read as UTF-16 code units) or `uXXXX` to
    /// `uXXXXXX` (one Unicode scalar value in uppercase hex). A single component that fails,
    /// as glyph indices such as `g12` or `cid12` do, makes the whole name `None`.
    pub fn text(&self) -> Option<Cow<'static, str>> {
        if let Some(text) = glyph_list::text(&self.0) {
            return Some(Cow::Borrowed(text));
        }

        let stem = self.0.split('.').next().unwrap_or_default();
        if stem.is_empty() {
            return None;
        }

        let mut components = stem.split('_').map(component_text);
        let first = components.next()??;
        components.try_fold(first, |mut text, component| {
            text.to_mut().push_str(&component?);
            Some(text)
        })
    }

    /// The name, without its leading `/`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&'static str> for GlyphName {
    fn from(name: &'static str) -> Self {
        Self(Cow::Borrowed(name))
    }
}

/// Invalid UTF-8 is kept lossily: such a name cannot be in the glyph list, so its text is `None`.
impl From<Name> for GlyphName {
    fn from(Name(bytes): Name) -> Self {
        let name = String::from_utf8(bytes)
            .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned());
        Self(Cow::Owned(name))
    }
}

/// The text of one component of a glyph name, by the list, then `uniXXXX`, then `uXXXX`.
fn component_text(component: &str) -> Option<Cow<'static, str>> {
    if let Some(text) = glyph_list::text(component) {
        return Some(Cow::Borrowed(text));
    }
    if let Some(units) = component.strip_prefix("uni") {
        if let Some(text) = uni(units) {
            return Some(Cow::Owned(text));
        }
    }
    let digits = component.strip_prefix('u')?;
    if !(4..=6).contains(&digits.len()) || !is_upper_hex(digits) {
        return None;
    }
    let scalar = char::from_u32(u32::from_str_radix(digits, 16).ok()?)?;
    Some(Cow::Owned(scalar.into()))
}

/// The text of the digits after `uni`: groups of four uppercase hex digits, each a UTF-16
/// code unit. Unpaired surrogates are refused.
fn uni(digits: &str) -> Option<String> {
    if digits.is_empty() || digits.len() % 4 != 0 || !is_upper_hex(digits) {
        return None;
    }
    let units = digits
        .as_bytes()
        .chunks(4)
        .map(|group| u16::from_str_radix(std::str::from_utf8(group).ok()?, 16).ok())
        .collect::<Option<Vec<u16>>>()?;
    String::from_utf16(&units).ok()
}

/// Whether `digits` are all `0-9` or `A-F`: the AGL specification refuses lowercase hex.
fn is_upper_hex(digits: &str) -> bool {
    digits
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case("A", Some("A"))]
    #[case("fi", Some("\u{FB01}"))]
    #[case("uni0041", Some("A"))]
    #[case("uni00660066", Some("ff"))]
    #[case("u1F600", Some("\u{1F600}"))]
    #[case("f_f_i", Some("ffi"))]
    #[case("A.sc", Some("A"))]
    #[case(".notdef", None)]
    #[case("g12", None)]
    #[case("cid12", None)]
    #[case("G12", None)]
    #[case("uni004", None)]
    #[case("uniD800", None)]
    fn text(#[case] name: &'static str, #[case] expected: Option<&str>) {
        assert_eq!(GlyphName::from(name).text().as_deref(), expected);
    }

    #[test]
    fn a_listed_name_is_borrowed() {
        assert!(matches!(
            GlyphName::from("A").text(),
            Some(Cow::Borrowed("A"))
        ));
    }

    #[test]
    fn invalid_utf8_has_no_text() {
        let name = GlyphName::from(Name(vec![b'A', 0xFF]));
        assert_eq!(name.text(), None);
    }
}
