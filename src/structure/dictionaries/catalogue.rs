use crate::parsers::{Extract, TypedReference};
use crate::serde::extract_deserialize;

use serde::Deserialize;

use super::super::PageNode;

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

    use crate::serde::extract_deserialize;
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

        let (_, Catalogue { pages }) = extract_deserialize(input).unwrap();
        assert_eq!(pages, TypedReference::new(2, 0));
    }
}
