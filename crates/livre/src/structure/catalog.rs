use winnow::{
    error::{ContextError, ErrMode},
    BStr, ModalResult,
};

use crate::{
    extraction::{extract, name_enum, RawDict, Reference},
    follow_refs::{Build, Builder},
};

use super::pages::PageTreeNode;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
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

name_enum!(PageLayout, "livre-page-layout":
    SinglePage => b"SinglePage",
    OneColumn => b"OneColumn",
    TwoColumnLeft => b"TwoColumnLeft",
    TwoColumnRight => b"TwoColumnRight",
    TwoPageLeft => b"TwoPageLeft",
    TwoPageRight => b"TwoPageRight",
);

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
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

name_enum!(PageMode, "livre-page-mode":
    UseNone => b"UseNone",
    UseOutlines => b"UseOutlines",
    UseThumbs => b"UseThumbs",
    FullScreen => b"FullScreen",
    UseOC => b"UseOC",
    UseAttachments => b"UseAttachments",
);

/// The document catalog contains references to other objects defining the content of the
/// document's content.
#[derive(Debug, PartialEq, Clone)]
pub struct Catalog {
    /// The root [page tree node](PageTreeNode).
    pub pages: PageTreeNode,

    /// A name object ([PageLayout]) specifying the page layout
    /// shall be used when the document is opened
    pub page_layout: PageLayout,

    /// A name object ([PageMode]) specifying how the document
    /// shall be displayed when opened
    pub page_mode: PageMode,
}

impl Build for Catalog {
    fn build<B>(input: &mut &BStr, builder: &B) -> ModalResult<Self>
    where
        B: Builder,
    {
        let mut dict: RawDict = extract(input)?;

        let page_layout = dict
            .pop(&"PageLayout".into())
            .map(|value| value.extract())
            .transpose()?
            .unwrap_or_default();

        let page_mode = dict
            .pop(&"PageMode".into())
            .map(|value| value.extract())
            .transpose()?
            .unwrap_or_default();

        let pages: Reference<PageTreeNode> = dict
            .pop(&"Pages".into())
            .ok_or(ErrMode::Cut(ContextError::new()))?
            .extract()?;

        let pages = builder.build_reference(pages)?;

        Ok(Self {
            page_mode,
            page_layout,
            pages,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, fmt::Debug};

    use rstest::rstest;

    use crate::extraction::{Extract, ReferenceId};

    use super::*;

    #[rstest]
    #[case::pages_not_a_reference(b"<</Type/Catalog/Pages 3>>")]
    #[case::pages_target_missing(b"<</Type/Catalog/Pages 3 0 R>>")]
    fn malformed_pages_is_an_error(#[case] input: &[u8]) {
        let objects: HashMap<ReferenceId, &BStr> = HashMap::new();
        assert!(Catalog::build(&mut input.as_ref(), &objects).is_err());
    }

    #[rstest]
    #[case(b"/SinglePage", PageLayout::SinglePage)]
    #[case(b"/OneColumn", PageLayout::OneColumn)]
    #[case(b"/TwoColumnLeft", PageLayout::TwoColumnLeft)]
    #[case(b"/TwoColumnRight", PageLayout::TwoColumnRight)]
    #[case(b"/TwoPageLeft", PageLayout::TwoPageLeft)]
    #[case(b"/TwoPageRight", PageLayout::TwoPageRight)]
    #[case(b"/UseNone", PageMode::UseNone)]
    #[case(b"/UseOutlines", PageMode::UseOutlines)]
    #[case(b"/UseThumbs", PageMode::UseThumbs)]
    #[case(b"/FullScreen", PageMode::FullScreen)]
    #[case(b"/UseOC", PageMode::UseOC)]
    #[case(b"/UseAttachments", PageMode::UseAttachments)]
    #[should_panic]
    #[case(b"/NotAVariant", PageMode::UseAttachments)]
    fn extraction<'de, T>(#[case] input: &'de [u8], #[case] expected: T)
    where
        T: Extract<'de> + Debug + PartialEq,
    {
        let res = extract(&mut input.as_ref()).unwrap();
        assert_eq!(expected, res)
    }
}
