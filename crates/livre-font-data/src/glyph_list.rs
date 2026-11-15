//! The [Adobe Glyph List], version 2.0 of September 20, 2002: the map from
//! glyph names to the Unicode text they stand for, from
//! `data/agl/glyphlist.txt`.
//!
//! [Adobe Glyph List]: https://github.com/adobe-type-tools/agl-aglfn

use crate::table::Table;

static GLYPH_LIST: Table<'static> = Table::new(include_bytes!("../data/agl/glyphlist.bin"));

/// The text a glyph name stands for, by the Adobe Glyph List. The list
/// only: the `uniXXXX` forms, suffixes and ligature components of the AGL
/// specification are rules the caller applies.
///
/// Some names map to several code points, so the result is a string rather
/// than a `char`.
///
/// ```
/// assert_eq!(livre_font_data::glyph_list::text("Adieresis"), Some("Ä"));
/// assert_eq!(livre_font_data::glyph_list::text("uni00C4"), None);
/// ```
pub fn text(name: &str) -> Option<&'static str> {
    std::str::from_utf8(GLYPH_LIST.get(name.as_bytes())?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("A", Some("A"))]
    #[case("Adieresis", Some("Ä"))]
    #[case("ffi", Some("\u{FB03}"))]
    #[case("dalethatafpatah", Some("\u{05D3}\u{05B2}"))]
    #[case("space", Some(" "))]
    #[case("uni0041", None)]
    #[case("notaglyph", None)]
    fn looks_up_a_name(#[case] name: &str, #[case] expected: Option<&str>) {
        assert_eq!(text(name), expected);
    }

    #[test]
    fn holds_every_entry_of_the_list() {
        let source = include_str!("../data/agl/glyphlist.txt");
        let entries = source
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .count();
        assert_eq!(GLYPH_LIST.len(), entries);
    }
}
