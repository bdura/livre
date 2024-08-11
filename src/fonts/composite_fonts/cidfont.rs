use std::fmt;

use nom::{branch::alt, combinator::map};
use serde::Deserialize;

use crate::{
    fonts::FontDescriptor,
    parsers::{parse, Extract, OptRef, TypedReference},
    serde::extract_deserialize,
    structure::Build,
};

#[derive(Debug, Deserialize, PartialEq)]
struct ArrayWidth(usize, Vec<u16>);

#[derive(Debug, Deserialize, PartialEq)]
struct RangeWidth(usize, usize, u16);

#[derive(Debug, PartialEq, Clone)]
pub enum WElement {
    Individual(usize, Vec<u16>),
    Range(usize, usize, u16),
}

impl Extract<'_> for WElement {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        alt((
            map(<(usize, Vec<u16>)>::extract, |(start, widths)| {
                Self::Individual(start, widths)
            }),
            map(<(usize, usize, u16)>::extract, |(start, stop, width)| {
                Self::Range(start, stop, width)
            }),
        ))(input)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Widths(pub Vec<WElement>);

impl Extract<'_> for Widths {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        let (input, elements) = Vec::<WElement>::extract(input)?;
        Ok((input, Self(elements)))
    }
}

struct WidthsVisitor;

impl<'de> serde::de::Visitor<'de> for WidthsVisitor {
    type Value = Widths;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a PDF CID font widths array")
    }

    fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        parse(value).map_err(E::custom)
    }
}

impl<'de> Deserialize<'de> for Widths {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_bytes(WidthsVisitor)
    }
}

impl WElement {
    pub fn width(&self, cid: usize) -> Option<u16> {
        match self {
            Self::Individual(start, values) => {
                if cid < *start {
                    None
                } else {
                    values.get(cid - start).copied()
                }
            }
            &Self::Range(start, stop, width) => {
                if cid < start || cid > stop {
                    None
                } else {
                    Some(width)
                }
            }
        }
    }
}

impl Widths {
    pub fn width(&self, cid: usize) -> Option<u16> {
        self.0
            .iter()
            .map(|e| e.width(cid))
            .find(Option::is_some)
            .flatten()
    }
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "PascalCase", default)]
pub struct CIDFontTypeTransient {
    base_font: String,
    #[serde(rename = "DW")]
    default_width: u16,
    #[serde(rename = "W")]
    widths: Option<OptRef<Widths>>,
    font_descriptor: TypedReference<FontDescriptor>,
}

impl Default for CIDFontTypeTransient {
    fn default() -> Self {
        Self {
            base_font: Default::default(),
            default_width: 1000,
            widths: None,
            font_descriptor: TypedReference::new(0, 0),
        }
    }
}

impl Extract<'_> for CIDFontTypeTransient {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        extract_deserialize(input)
    }
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct CIDFontType {
    pub base_font: String,
    pub default_width: u16,
    pub widths: Option<Widths>,
    pub font_descriptor: FontDescriptor,
}

impl CIDFontType {
    pub fn width(&self, cid: usize) -> u16 {
        if let Some(widths) = &self.widths {
            widths.width(cid).unwrap_or(self.default_width)
        } else {
            self.default_width
        }
    }
}

impl Build for CIDFontTypeTransient {
    type Output = CIDFontType;

    fn build(self, doc: &crate::structure::Document) -> Self::Output {
        let Self {
            base_font,
            default_width,
            widths,
            font_descriptor,
        } = self;

        let font_descriptor = font_descriptor.build(doc);
        let widths = widths.map(|e| e.build(doc));

        CIDFontType {
            base_font,
            default_width,
            widths,
            font_descriptor,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use crate::serde::from_bytes;

    use super::*;

    use rstest::rstest;

    #[rstest]
    #[case(WElement::Individual(1, vec![1, 2]), 0, None)]
    #[case(WElement::Individual(1, vec![1, 2]), 1, Some(1))]
    #[case(WElement::Individual(1, vec![1, 2]), 2, Some(2))]
    #[case(WElement::Individual(1, vec![1, 2]), 3, None)]
    #[case(WElement::Range(1, 2, 1), 0, None)]
    #[case(WElement::Range(1, 2, 1), 1, Some(1))]
    #[case(WElement::Range(1, 2, 1), 2, Some(1))]
    #[case(WElement::Range(1, 2, 1), 3, None)]
    fn width(#[case] element: WElement, #[case] cid: usize, #[case] expected: Option<u16>) {
        assert_eq!(expected, element.width(cid))
    }

    #[rstest]
    #[case(b"0 [10]", WElement::Individual(0, vec![10]))]
    #[case(b"0[10]", WElement::Individual(0, vec![10]))]
    #[case(b"[0 [10] 0[10]]", vec![WElement::Individual(0, vec![10]), WElement::Individual(0, vec![10])])]
    #[case(b"0 10 10", WElement::Range(0, 10, 10))]
    #[case(b"[0 10 10 0 10 10]", vec![WElement::Range(0, 10, 10), WElement::Range(0, 10, 10)])]
    #[case(b"[0 10 10 0 10 10]", Widths(vec![WElement::Range(0, 10, 10), WElement::Range(0, 10, 10)]))]
    #[case(b"[0 10 10 0[10]]", Widths(vec![WElement::Range(0, 10, 10), WElement::Individual(0, vec![10])]))]
    fn extraction<'de, T: Extract<'de> + PartialEq + Debug>(
        #[case] input: &'de [u8],
        #[case] expected: T,
    ) {
        assert_eq!(expected, parse(input).unwrap());
    }

    #[rstest]
    #[case(b"[0 10 10 0 10 10]", Widths(vec![WElement::Range(0, 10, 10), WElement::Range(0, 10, 10)]))]
    #[case(b"[0 10 10 0[10]]", Widths(vec![WElement::Range(0, 10, 10), WElement::Individual(0, vec![10])]))]
    fn deserialization<'de, T: Deserialize<'de> + PartialEq + Debug>(
        #[case] input: &'de [u8],
        #[case] expected: T,
    ) {
        assert_eq!(expected, from_bytes(input).unwrap());
    }
}
