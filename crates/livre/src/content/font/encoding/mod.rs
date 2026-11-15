//! Encodings of simple fonts: from a one-byte code to a glyph name (ISO 32000-2, 9.6.5).
//!
//! A font's [`Encoding`] starts from a built-in table, chosen from its `/Encoding`, its
//! `/BaseFont` and its descriptor's `/Flags`, then applies the `/Differences` of its encoding
//! dictionary.

mod differences;
mod glyph_name;
mod tables;

use winnow::{BStr, ModalResult};

pub(crate) use differences::{Differences, EncodingSpec};
pub use glyph_name::GlyphName;
pub(crate) use tables::Table;
use tables::{MAC_EXPERT, MAC_ROMAN, STANDARD, SYMBOL, WIN_ANSI, ZAPF_DINGBATS};

use super::Code;
use crate::{
    extraction::{extract, Extract, Name},
    follow_refs::{Build, Builder},
};

/// The font descriptor's `/Flags` (ISO 32000-2, 9.8.2, Table 121). Defaults to no flag set.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Extract)]
pub struct Flags(
    /// The raw bit field, bit 1 being the least significant.
    pub u32,
);

impl Flags {
    /// Bit 3: the font uses glyphs outside the standard Latin character set.
    const SYMBOLIC: u32 = 1 << 2;
    /// Bit 6: the font's glyphs are within the standard Latin character set.
    const NONSYMBOLIC: u32 = 1 << 5;

    /// Whether the Symbolic flag is set.
    pub fn is_symbolic(self) -> bool {
        self.0 & Self::SYMBOLIC != 0
    }

    /// Whether the Nonsymbolic flag is set.
    pub fn is_nonsymbolic(self) -> bool {
        self.0 & Self::NONSYMBOLIC != 0
    }
}

impl Build for Flags {
    fn build<B>(input: &mut &BStr, _builder: &B) -> ModalResult<Self>
    where
        B: Builder,
    {
        extract(input)
    }
}

/// The table a predefined encoding name denotes.
fn named_table(name: &Name) -> Option<&'static Table> {
    match name.0.as_slice() {
        b"StandardEncoding" => Some(&STANDARD),
        b"WinAnsiEncoding" => Some(&WIN_ANSI),
        b"MacRomanEncoding" => Some(&MAC_ROMAN),
        b"MacExpertEncoding" => Some(&MAC_EXPERT),
        _ => None,
    }
}

/// `/BaseFont` without its subset tag: six uppercase letters and a `+` (ISO 32000-2, 9.9.2).
fn strip_subset_tag(base_font: &[u8]) -> &[u8] {
    match base_font.split_at_checked(7) {
        Some((tag, rest)) if tag[6] == b'+' && tag[..6].iter().all(u8::is_ascii_uppercase) => rest,
        _ => base_font,
    }
}

/// Which built-in table the font's codes start from, before `/Differences`.
///
/// In order: the predefined encoding `spec` names, directly or as `/BaseEncoding`; the Symbol
/// or ZapfDingbats table for those fonts; nothing for a font flagged symbolic, whose built-in
/// encoding is in its program; else `StandardEncoding`. An unknown encoding name and a
/// symbolic font without a table both warn.
pub(crate) fn base_table(
    spec: Option<&EncodingSpec>,
    base_font: &Name,
    flags: Flags,
) -> Option<&'static Table> {
    let font = String::from_utf8_lossy(base_font);

    let name = match spec {
        Some(EncodingSpec::Named(name)) => Some(name),
        Some(EncodingSpec::Dictionary(dict)) => dict.base_encoding.as_ref(),
        None => None,
    };
    if let Some(name) = name {
        match named_table(name) {
            Some(table) => return Some(table),
            None => tracing::warn!(
                "Font {font}: unknown /Encoding /{}",
                String::from_utf8_lossy(name)
            ),
        }
    }

    match strip_subset_tag(base_font) {
        b"Symbol" | b"SymbolMT" => return Some(&SYMBOL),
        b"ZapfDingbats" | b"Dingbats" => return Some(&ZAPF_DINGBATS),
        _ => {}
    }

    if flags.is_symbolic() && !flags.is_nonsymbolic() {
        tracing::warn!(
            "symbolic font {font} without /Encoding: built-in encoding unknown, \
             relying on /Differences and /ToUnicode"
        );
        return None;
    }

    Some(&STANDARD)
}

/// The code-to-glyph-name map of a simple font, base table and differences applied.
#[derive(Debug, Clone, PartialEq)]
pub struct Encoding {
    names: Box<[Option<GlyphName>; 256]>,
}

impl Encoding {
    /// The names of `base`, if any, overwritten by `differences`.
    pub(crate) fn new(base: Option<&'static Table>, differences: &Differences) -> Self {
        let mut names: Box<[Option<GlyphName>; 256]> = Box::new(std::array::from_fn(|code| {
            base.and_then(|table| table[code]).map(GlyphName::from)
        }));
        for (code, name) in differences.iter() {
            names[usize::from(code)] = Some(name.clone());
        }
        Self { names }
    }

    /// The glyph name of `code`, if the encoding has one. Always `None` for a code that is not
    /// one byte long.
    pub fn glyph_name(&self, code: Code) -> Option<&GlyphName> {
        if code.byte_count() != 1 {
            return None;
        }
        self.names
            .get(usize::try_from(code.value()).ok()?)?
            .as_ref()
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use rstest::rstest;

    use super::*;

    fn spec(input: &[u8]) -> EncodingSpec {
        let mut input: &BStr = input.as_ref();
        Build::build(&mut input, &()).unwrap()
    }

    fn same(left: Option<&'static Table>, right: Option<&'static Table>) -> bool {
        match (left, right) {
            (Some(left), Some(right)) => std::ptr::eq(left, right),
            (None, None) => true,
            _ => false,
        }
    }

    #[tracing_test::traced_test]
    #[rstest]
    #[case::named(Some(&b"/WinAnsiEncoding"[..]), "Helvetica", 0, Some(&WIN_ANSI))]
    #[case::base_encoding(
        Some(&b"<</BaseEncoding/MacRomanEncoding/Differences[39/quotesingle]>>"[..]),
        "Helvetica",
        0,
        Some(&MAC_ROMAN)
    )]
    #[case::symbol(None, "Symbol", 0, Some(&SYMBOL))]
    #[case::subset_dingbats(None, "ABCDEF+ZapfDingbats", 0, Some(&ZAPF_DINGBATS))]
    #[case::nonsymbolic(None, "Arial", 32, Some(&STANDARD))]
    #[case::no_flags(None, "Arial", 0, Some(&STANDARD))]
    #[case::dictionary_without_base(Some(&b"<</Differences[39/quotesingle]>>"[..]), "Arial", 0, Some(&STANDARD))]
    fn base_table_branches(
        #[case] input: Option<&[u8]>,
        #[case] base_font: &str,
        #[case] flags: u32,
        #[case] expected: Option<&'static Table>,
    ) {
        let spec = input.map(spec);
        let table = base_table(spec.as_ref(), &base_font.into(), Flags(flags));

        assert!(same(table, expected), "{table:?}");
        assert!(!logs_contain("WARN"));
    }

    #[test]
    #[tracing_test::traced_test]
    fn unknown_name_warns_and_falls_through() {
        let table = base_table(
            Some(&spec(b"/FooEncoding")),
            &"Helvetica".into(),
            Flags::default(),
        );

        assert!(same(table, Some(&STANDARD)));
        assert!(logs_contain("unknown /Encoding /FooEncoding"));
    }

    #[test]
    #[tracing_test::traced_test]
    fn symbolic_without_encoding_has_no_base() {
        let table = base_table(None, &"ABCDEF+Wingdings".into(), Flags(4));

        assert!(table.is_none());
        assert!(logs_contain(
            "symbolic font ABCDEF+Wingdings without /Encoding"
        ));
    }

    #[test]
    fn flags_from_a_descriptor_value() {
        let flags: Flags = extract(&mut b"36".as_slice().as_ref()).unwrap();
        assert!(flags.is_symbolic());
        assert!(flags.is_nonsymbolic());
    }

    #[test]
    fn differences_over_a_table() {
        let differences: Differences =
            extract(&mut b"[39/quotesingle]".as_slice().as_ref()).unwrap();
        let encoding = Encoding::new(Some(&STANDARD), &differences);

        let quotesingle = encoding.glyph_name(Code::one_byte(39)).unwrap();
        assert_eq!(quotesingle.as_str(), "quotesingle");
        assert!(matches!(quotesingle.0, Cow::Owned(_)));

        let a = encoding.glyph_name(Code::one_byte(65)).unwrap();
        assert_eq!(a.as_str(), "A");
        assert!(matches!(a.0, Cow::Borrowed(_)));

        assert_eq!(encoding.glyph_name(Code::one_byte(0)), None);
    }

    #[test]
    fn no_base_keeps_only_the_differences() {
        let differences: Differences = extract(&mut b"[97/alpha]".as_slice().as_ref()).unwrap();
        let encoding = Encoding::new(None, &differences);

        assert_eq!(
            encoding
                .glyph_name(Code::one_byte(97))
                .map(GlyphName::as_str),
            Some("alpha")
        );
        assert_eq!(encoding.glyph_name(Code::one_byte(65)), None);
    }
}
