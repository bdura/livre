use livre_extraction::{parse, Extract, FromDict, RawDict};
use livre_objects::Name;

use crate::{PageLeaf, PageNode};

#[derive(Debug, PartialEq, Clone)]
pub enum PageElement {
    Node(PageNode),
    Leaf(PageLeaf),
}

impl Extract<'_> for PageElement {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        let (input, dict) = RawDict::extract(input)?;

        let Name(variant) = parse(dict.get("Type").unwrap().0).unwrap();

        println!("{variant}");

        let element = if variant == "Pages" {
            let node = PageNode::from_dict(dict).unwrap();
            PageElement::Node(node)
        } else {
            let leaf = PageLeaf::from_dict(dict).unwrap();
            PageElement::Leaf(leaf)
        };

        Ok((input, element))
    }
}

impl PageElement {
    pub fn is_node(&self) -> bool {
        matches!(self, PageElement::Node(_))
    }
    pub fn is_leaf(&self) -> bool {
        matches!(self, PageElement::Leaf(_))
    }
}

#[cfg(test)]
mod tests {

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
        let (_, page) = PageElement::extract(input).unwrap();
        assert_eq!(page.is_leaf(), is_leaf);
    }
}
