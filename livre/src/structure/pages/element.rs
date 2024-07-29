use crate::parsers::Extract;
use crate::serde::extract_deserialize;
use serde::Deserialize;

use super::{PageLeaf, PageNode};

#[derive(Debug, PartialEq, Clone, Deserialize)]
#[serde(tag = "Type")]
pub enum PageElement {
    Pages(PageNode),
    Page(PageLeaf),
}

impl PageElement {
    pub fn is_node(&self) -> bool {
        matches!(self, PageElement::Pages(_))
    }
    pub fn is_leaf(&self) -> bool {
        matches!(self, PageElement::Page(_))
    }
}

impl Extract<'_> for PageElement {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        extract_deserialize(input)
    }
}

#[cfg(test)]
mod tests {

    use crate::objects::Bytes;
    use crate::serde::extract_deserialize;
    use indoc::indoc;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(
        indoc! {b"
            <<
                /Type /Page
                /Parent 4 0 R
                /MediaBox [0 0 612 792]
                /Resources  <<
                    /Font <<
                        /F3 7 0 R
                        /F5 9 0 R
                        /F7 11 0 R 
                    >>
                >>
                /Contents 12 0 R
                /Annots [23 0 R
                24 0 R
                ]
            >>
        "},
        true
    )]
    #[case(
        indoc! {b"
            <</Type /Pages
            /Kids [4 0 R
                10 0 R
                24 0 R
            ] /Count 3
            >>
        "},
        false
    )]
    fn page(#[case] input: &[u8], #[case] is_leaf: bool) {
        let (_, page) = extract_deserialize::<PageElement>(input)
            .map_err(|e| e.map_input(Bytes::from))
            .unwrap();
        assert_eq!(page.is_leaf(), is_leaf);
    }
}
