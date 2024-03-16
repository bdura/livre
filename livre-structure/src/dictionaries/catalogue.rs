use livre_extraction::{Extract, FromDictRef, TypedReference};

use crate::PageNode;

#[derive(Debug, PartialEq, FromDictRef, Extract, Clone)]
pub struct Catalogue {
    // pub version: Option<Name>,
    // pub extensions
    pub pages: TypedReference<PageNode>,
}

#[cfg(test)]
mod tests {

    use indoc::indoc;

    use super::*;

    #[test]
    fn catalogue() {
        let input = indoc! {b"
            <</Type /Catalog
                /Pages 2 0 R
                /PageMode /UseOutlines
                /Outlines 3 0 R
            >>
        "};

        let (_, catalogue) = Catalogue::extract(input).unwrap();
        assert_eq!(catalogue.pages, TypedReference::new(2, 0));
    }
}
