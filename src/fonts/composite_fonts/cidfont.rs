use serde::Deserialize;

use crate::parsers::OptRef;

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub enum WElement {
    Individual(usize, Vec<u16>),
    /// Inclusive range
    Range(usize, usize, u16),
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

fn find_width(cid: usize, w: &[WElement]) -> Option<u16> {
    w.iter()
        .map(|e| e.width(cid))
        .find(Option::is_some)
        .flatten()
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "PascalCase", default)]
pub struct CIDFontType {
    base_font: String,
    #[serde(rename = "DW")]
    default_width: u16,
    #[serde(rename = "W")]
    widths: Option<OptRef<Vec<WElement>>>,
}

impl Default for CIDFontType {
    fn default() -> Self {
        Self {
            base_font: Default::default(),
            default_width: 1000,
            widths: None,
        }
    }
}

#[cfg(test)]
mod tests {
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
}
