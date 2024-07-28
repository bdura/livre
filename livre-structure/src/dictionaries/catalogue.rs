use livre_extraction::{Extract, TypedReference};
use livre_serde::extract_deserialize;
use serde::Deserialize;

use crate::PageNode;

#[derive(Debug, PartialEq, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Catalogue {
    // pub version: Option<Name>,
    // pub extensions
    pub pages: TypedReference<PageNode>,
}

impl Extract<'_> for Catalogue {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        extract_deserialize(input)
    }
}

#[cfg(test)]
mod tests {

    use indoc::indoc;
    use livre_serde::extract_deserialize;

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

        let (_, Catalogue { pages }) = extract_deserialize(input).unwrap();
        assert_eq!(pages, TypedReference::new(2, 0));
    }
}
