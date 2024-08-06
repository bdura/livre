use crate::parsers::{Extract, TypedReference};
use crate::serde::extract_deserialize;

use serde::Deserialize;

use super::super::PageNode;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize, Default)]
pub enum PageLayout {
    ///Display one page at a time
    #[default]
    SinglePage,
    /// Display the pages in one column
    OneColumn,
    /// Display the pages in two columns, with odd-
    /// numbered pages on the left
    TwoColumnLeft,
    /// Display the pages in two columns, with odd-
    /// numbered pages on the right
    TwoColumnRight,
    /// (PDF 1.5) Display the pages two at a time, with
    /// odd-numbered pages on the left
    TwoPageLeft,
    /// (PDF 1.5) Display the pages two at a time, with
    /// odd-numbered pages on the right
    TwoPageRight,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize, Default)]
pub enum PageMode {
    /// Neither document outline nor thumbnail images visible
    #[default]
    UseNone,
    /// Document outline visible
    UseOutlines,
    /// Thumbnail images visible
    UseThumbs,
    /// Full-screen mode, with no menu bar, window controls, or any other window visible
    FullScreen,
    /// (PDF 1.5) Optional content group panel visible
    UseOC,
    /// (PDF 1.6) Attachments panel visible
    UseAttachments,
}

#[derive(Debug, PartialEq, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Catalogue {
    // pub version: Option<Name>,
    // pub extensions
    /// The root [page tree node](PageNode)
    pub pages: TypedReference<PageNode>,
    /// A name object ([PageLayout]) specifying the page layout
    /// shall be used when the document is opened
    #[serde(default)]
    pub page_layout: PageLayout,
    /// A name object ([PageMode]) specifying how the document
    /// shall be displayed when opened
    #[serde(default)]
    pub page_mode: PageMode,
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

        let (_, Catalogue { pages, .. }) = extract_deserialize(input).unwrap();
        assert_eq!(pages, TypedReference::new(2, 0));
    }
}
