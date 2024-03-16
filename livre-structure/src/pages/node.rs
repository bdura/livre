use livre_extraction::{Extract, FromDictRef, Reference, TypedReference};

use crate::PageElement;

use super::props::PageProperties;

#[derive(Debug, PartialEq, Clone, FromDictRef, Extract)]
pub struct PageNode {
    pub parent: Option<Reference>,
    pub kids: Vec<TypedReference<PageElement>>,
    #[livre(flatten)]
    pub props: PageProperties,
}

#[cfg(test)]
mod tests {

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

        let (_, node) = PageNode::extract(input).unwrap();
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
