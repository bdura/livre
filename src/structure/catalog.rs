use winnow::{
    combinator::{fail, trace},
    error::{ContextError, ErrMode},
    BStr, PResult, Parser,
};

use crate::{
    extraction::{extract, Extract, Name, RawDict, Reference},
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

impl Extract<'_> for PageLayout {
    fn extract(input: &mut &BStr) -> PResult<Self> {
        trace("livre-page-layout", move |i: &mut &BStr| {
            let Name(value) = extract(i)?;

            let res = match value.as_slice() {
                b"SinglePage" => Self::SinglePage,
                b"OneColumn" => Self::OneColumn,
                b"TwoColumnLeft" => Self::TwoColumnLeft,
                b"TwoColumnRight" => Self::TwoColumnRight,
                b"TwoPageLeft" => Self::TwoPageLeft,
                b"TwoPageRight" => Self::TwoPageRight,
                _ => fail(i)?,
            };

            Ok(res)
        })
        .parse_next(input)
    }
}

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

impl Extract<'_> for PageMode {
    fn extract(input: &mut &BStr) -> PResult<Self> {
        trace("livre-page-mode", move |i: &mut &BStr| {
            let Name(value) = extract(i)?;

            let res = match value.as_slice() {
                b"UseNone" => Self::UseNone,
                b"UseOutlines" => Self::UseOutlines,
                b"UseThumbs" => Self::UseThumbs,
                b"FullScreen" => Self::FullScreen,
                b"UseOC" => Self::UseOC,
                b"UseAttachments" => Self::UseAttachments,
                _ => fail(i)?,
            };

            Ok(res)
        })
        .parse_next(input)
    }
}

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
    fn build<B>(input: &mut &BStr, builder: &B) -> PResult<Self>
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
            .extract()
            .unwrap();

        let pages = builder.build_reference(pages).unwrap();

        Ok(Self {
            page_mode,
            page_layout,
            pages,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use rstest::rstest;

    use super::*;

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

    //struct DummyBuilder(HashMap<ReferenceId, &'static BStr>);
    //
    //impl DummyBuilder {
    //    fn new_simple<I: Into<ReferenceId>>(key: I, value: &'static BStr) -> Self {
    //        let mut map = HashMap::new();
    //        map.insert(key.into(), value);
    //
    //        Self(map)
    //    }
    //}
    //
    //impl Builder for DummyBuilder {
    //    fn build_reference<T>(&self, reference: Reference<T>) -> PResult<T>
    //    where
    //        T: Build,
    //    {
    //        let mut input = self
    //            .0
    //            .get(&reference.id)
    //            .copied()
    //            .ok_or(ErrMode::Cut(ContextError::new()))?;
    //
    //        self.build(&mut input)
    //    }
    //}
    //
    //#[rstest]
    //#[case(
    //    indoc! {b"
    //        <</Type /Catalog
    //            /Pages 2 0 R
    //            /PageMode /UseOutlines
    //            /Outlines 3 0 R
    //        >>
    //    "},
    //    Catalog {
    //        page_mode: PageMode::UseOutlines,
    //        page_layout: PageLayout::default(),
    //        pages: Pages(vec![])
    //    }
    //)]
    //fn building<T>(#[case] input: &[u8], #[case] expected: T)
    //where
    //    T: Build + Debug + PartialEq,
    //{
    //    let builder = DummyBuilder::new_simple((2, 0), b"".as_ref().into());
    //    let res = T::build(&mut input.as_ref(), &builder).unwrap();
    //
    //    assert_eq!(expected, res);
    //}
}
