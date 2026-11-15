//! The fourteen standard Type 1 fonts (ISO 32000-1:2008, §9.6.2.2), with the
//! glyph widths of Adobe's Core 14 AFM files in `data/afm/`.

use crate::table::Table;

/// The fourteen fonts a conforming reader must know (ISO 32000-1:2008,
/// §9.6.2.2), with Adobe's Core 14 metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StandardFont {
    Courier,
    CourierBold,
    CourierOblique,
    CourierBoldOblique,
    Helvetica,
    HelveticaBold,
    HelveticaOblique,
    HelveticaBoldOblique,
    TimesRoman,
    TimesBold,
    TimesItalic,
    TimesBoldItalic,
    Symbol,
    ZapfDingbats,
}

/// The families [`StandardFont::from_base_font`] recognises.
#[derive(Clone, Copy)]
enum Family {
    Courier,
    Helvetica,
    Times,
    Symbol,
    ZapfDingbats,
}

impl StandardFont {
    /// Every standard font, in declaration order.
    pub const ALL: [Self; 14] = [
        Self::Courier,
        Self::CourierBold,
        Self::CourierOblique,
        Self::CourierBoldOblique,
        Self::Helvetica,
        Self::HelveticaBold,
        Self::HelveticaOblique,
        Self::HelveticaBoldOblique,
        Self::TimesRoman,
        Self::TimesBold,
        Self::TimesItalic,
        Self::TimesBoldItalic,
        Self::Symbol,
        Self::ZapfDingbats,
    ];

    /// The font a `/BaseFont` name denotes, if it is one of the fourteen or
    /// one of their usual aliases: Arial for Helvetica, Times New Roman for
    /// Times, Courier New for Courier, with the `PS`, `MT` and `PSMT`
    /// suffixes and the `,Bold` or `-BoldItalic` style spellings producers
    /// write. A subset prefix such as `ABCDEF+` is ignored.
    ///
    /// Case-sensitive: these are PostScript names, and `arial` is not one.
    ///
    /// ```
    /// use livre_font_data::StandardFont;
    /// assert_eq!(StandardFont::from_base_font("ABCDEF+Arial,Bold"), Some(StandardFont::HelveticaBold));
    /// assert_eq!(StandardFont::from_base_font("Calibri"), None);
    /// ```
    pub fn from_base_font(name: &str) -> Option<Self> {
        let name = strip_subset_prefix(name);
        let (family, style) = name
            .find([',', '-'])
            .map_or((name, ""), |at| (&name[..at], &name[at + 1..]));
        let family = ["PSMT", "PS", "MT"]
            .iter()
            .find_map(|suffix| family.strip_suffix(suffix))
            .unwrap_or(family);
        let family = match family {
            "Arial" | "Helvetica" => Family::Helvetica,
            "TimesNewRoman" | "Times" => Family::Times,
            "CourierNew" | "Courier" => Family::Courier,
            "Symbol" => Family::Symbol,
            "ZapfDingbats" | "Dingbats" => Family::ZapfDingbats,
            _ => return None,
        };
        let bold = style.contains("Bold");
        let italic = style.contains("Italic") || style.contains("Oblique");
        Some(match (family, bold, italic) {
            (Family::Courier, false, false) => Self::Courier,
            (Family::Courier, true, false) => Self::CourierBold,
            (Family::Courier, false, true) => Self::CourierOblique,
            (Family::Courier, true, true) => Self::CourierBoldOblique,
            (Family::Helvetica, false, false) => Self::Helvetica,
            (Family::Helvetica, true, false) => Self::HelveticaBold,
            (Family::Helvetica, false, true) => Self::HelveticaOblique,
            (Family::Helvetica, true, true) => Self::HelveticaBoldOblique,
            (Family::Times, false, false) => Self::TimesRoman,
            (Family::Times, true, false) => Self::TimesBold,
            (Family::Times, false, true) => Self::TimesItalic,
            (Family::Times, true, true) => Self::TimesBoldItalic,
            (Family::Symbol, _, _) => Self::Symbol,
            (Family::ZapfDingbats, _, _) => Self::ZapfDingbats,
        })
    }

    /// The advance width of the glyph named `glyph`, in thousandths of an
    /// em, from the font's AFM file.
    ///
    /// ```
    /// use livre_font_data::StandardFont;
    /// assert_eq!(StandardFont::Helvetica.width("A"), Some(667));
    /// ```
    pub fn width(self, glyph: &str) -> Option<u16> {
        let value = self.table().get(glyph.as_bytes())?;
        Some(u16::from_le_bytes(value.try_into().ok()?))
    }

    /// The PostScript name, as the AFM's `FontName`; also the stem of its
    /// files in `data/afm/`.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Courier => "Courier",
            Self::CourierBold => "Courier-Bold",
            Self::CourierOblique => "Courier-Oblique",
            Self::CourierBoldOblique => "Courier-BoldOblique",
            Self::Helvetica => "Helvetica",
            Self::HelveticaBold => "Helvetica-Bold",
            Self::HelveticaOblique => "Helvetica-Oblique",
            Self::HelveticaBoldOblique => "Helvetica-BoldOblique",
            Self::TimesRoman => "Times-Roman",
            Self::TimesBold => "Times-Bold",
            Self::TimesItalic => "Times-Italic",
            Self::TimesBoldItalic => "Times-BoldItalic",
            Self::Symbol => "Symbol",
            Self::ZapfDingbats => "ZapfDingbats",
        }
    }

    /// The glyph-name-to-width table, values two little-endian bytes.
    const fn table(self) -> Table<'static> {
        // `include_bytes!` takes a literal, hence one line per font.
        Table::new(match self {
            Self::Courier => include_bytes!("../data/afm/Courier.bin"),
            Self::CourierBold => include_bytes!("../data/afm/Courier-Bold.bin"),
            Self::CourierOblique => include_bytes!("../data/afm/Courier-Oblique.bin"),
            Self::CourierBoldOblique => include_bytes!("../data/afm/Courier-BoldOblique.bin"),
            Self::Helvetica => include_bytes!("../data/afm/Helvetica.bin"),
            Self::HelveticaBold => include_bytes!("../data/afm/Helvetica-Bold.bin"),
            Self::HelveticaOblique => include_bytes!("../data/afm/Helvetica-Oblique.bin"),
            Self::HelveticaBoldOblique => include_bytes!("../data/afm/Helvetica-BoldOblique.bin"),
            Self::TimesRoman => include_bytes!("../data/afm/Times-Roman.bin"),
            Self::TimesBold => include_bytes!("../data/afm/Times-Bold.bin"),
            Self::TimesItalic => include_bytes!("../data/afm/Times-Italic.bin"),
            Self::TimesBoldItalic => include_bytes!("../data/afm/Times-BoldItalic.bin"),
            Self::Symbol => include_bytes!("../data/afm/Symbol.bin"),
            Self::ZapfDingbats => include_bytes!("../data/afm/ZapfDingbats.bin"),
        })
    }
}

/// `name` without its subset tag, six ASCII uppercase letters and a `+`
/// (ISO 32000-1:2008, §9.6.4).
fn strip_subset_prefix(name: &str) -> &str {
    match name.split_once('+') {
        Some((tag, rest)) if tag.len() == 6 && tag.bytes().all(|b| b.is_ascii_uppercase()) => rest,
        _ => name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use StandardFont::*;

    #[rstest]
    #[case::helvetica("Helvetica", Some(Helvetica))]
    #[case::helvetica("Arial", Some(Helvetica))]
    #[case::helvetica("ArialMT", Some(Helvetica))]
    #[case::helvetica("ABCDEF+Arial", Some(Helvetica))]
    #[case::helvetica_bold("Arial,Bold", Some(HelveticaBold))]
    #[case::helvetica_bold("Arial-BoldMT", Some(HelveticaBold))]
    #[case::helvetica_bold("Helvetica-Bold", Some(HelveticaBold))]
    #[case::helvetica_bold_oblique("Arial-BoldItalicMT", Some(HelveticaBoldOblique))]
    #[case::helvetica_bold_oblique("Helvetica-BoldOblique", Some(HelveticaBoldOblique))]
    #[case::helvetica_oblique("Arial,Italic", Some(HelveticaOblique))]
    #[case::times_roman("Times-Roman", Some(TimesRoman))]
    #[case::times_roman("TimesNewRoman", Some(TimesRoman))]
    #[case::times_roman("TimesNewRomanPSMT", Some(TimesRoman))]
    #[case::times_bold_italic("TimesNewRomanPS-BoldItalicMT", Some(TimesBoldItalic))]
    #[case::courier("Courier", Some(Courier))]
    #[case::courier("CourierNew", Some(Courier))]
    #[case::courier("CourierNewPSMT", Some(Courier))]
    #[case::courier_bold("CourierNew,Bold", Some(CourierBold))]
    #[case::symbol("Symbol", Some(Symbol))]
    #[case::symbol("SymbolMT", Some(Symbol))]
    #[case::symbol("Symbol,Bold", Some(Symbol))]
    #[case::zapf_dingbats("ZapfDingbats", Some(ZapfDingbats))]
    #[case::none("Calibri", None)]
    #[case::none("ABCDEF+Calibri", None)]
    #[case::none("arial", None)]
    #[case::none("", None)]
    fn resolves_a_base_font(#[case] name: &str, #[case] expected: Option<StandardFont>) {
        assert_eq!(StandardFont::from_base_font(name), expected);
    }

    #[rstest]
    #[case(Helvetica, "A", Some(667))]
    #[case(Helvetica, "space", Some(278))]
    #[case(HelveticaBold, "A", Some(722))]
    #[case(TimesRoman, "A", Some(722))]
    #[case(Courier, "A", Some(600))]
    #[case(Courier, "W", Some(600))]
    #[case(Symbol, "alpha", Some(631))]
    #[case(ZapfDingbats, "a1", Some(974))]
    #[case(Helvetica, "notaglyph", None)]
    fn reads_a_width(
        #[case] font: StandardFont,
        #[case] glyph: &str,
        #[case] expected: Option<u16>,
    ) {
        assert_eq!(font.width(glyph), expected);
    }
}
