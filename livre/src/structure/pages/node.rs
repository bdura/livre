use crate::parsers::{Extract, Reference, TypedReference};
use crate::serde::extract_deserialize;
use serde::Deserialize;

use super::PageElement;

use super::props::PageProperties;

#[derive(Debug, PartialEq, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PageNode {
    pub parent: Option<Reference>,
    pub kids: Vec<TypedReference<PageElement>>,
    #[serde(flatten)]
    pub props: PageProperties,
}

impl Extract<'_> for PageNode {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        extract_deserialize(input)
    }
}

#[cfg(test)]
mod tests {

    use crate::serde::extract_deserialize;
    use indoc::indoc;

    use super::*;

    #[test]
    fn node() {
        let input = indoc! {b"
            <</Type /Pages
                /Kids [4 0 R
                    10 0 R
                    24 0 R
                ] /Count 3
            >>
        "};

        let (_, node) = extract_deserialize::<PageNode>(input).unwrap();
        assert_eq!(
            node.kids,
            vec![
                TypedReference::new(4, 0),
                TypedReference::new(10, 0),
                TypedReference::new(24, 0)
            ]
        );
    }
}
