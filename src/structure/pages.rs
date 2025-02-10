use std::collections::HashMap;

use winnow::{
    error::{ContextError, ErrMode},
    BStr, PResult, Parser,
};

use crate::{
    extraction::{
        extract, Extract, FromRawDict, MaybeArray, Name, OptRef, RawDict, Rectangle, Reference,
        Stream,
    },
    follow_refs::{Build, Builder},
};

/// Page resources.
///
/// Livre focuses on fonts for now.
#[derive(Debug, PartialEq, Clone, FromRawDict)]
pub struct Resources {
    /// Font dictionary.
    pub font: HashMap<Name, Reference<()>>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum RotationAngle {
    #[default]
    Zero,
    Quarter,
    Full,
    ThreeQuarters,
}

impl Extract<'_> for RotationAngle {
    fn extract(input: &mut &'_ BStr) -> PResult<Self> {
        extract
            .verify_map(|i: u16| match i {
                0 => Some(Self::Zero),
                90 => Some(Self::Quarter),
                180 => Some(Self::Full),
                270 => Some(Self::ThreeQuarters),
                _ => None,
            })
            .parse_next(input)
    }
}

impl Build for RotationAngle {
    fn build<B>(input: &mut &BStr, _builder: &B) -> PResult<Self>
    where
        B: Builder,
    {
        extract(input)
    }
}

/// Inheritable page properties. Each attribute shall be inherited as-is, without merging.
///
/// Thus for instance the [Resources] dictionary for a page shall be
/// found by searching the [PageLeaf](super::PageLeaf) object and then
/// each [PageNode](super::PageNode) object encountered by following
/// `Parent`` links up the pages tree from the Page towards the Catalog object.
/// When the first Resources dictionary is found the search shall be stopped
/// and that Resources dictionary shall be used in its entirety.
#[derive(Debug, PartialEq, Clone, FromRawDict)]
pub struct InheritablePageProperties {
    /// A dictionary containing any resources required by the page contents.
    pub resources: Option<OptRef<Resources>>,
    /// A rectangle expressed in default user space units, that shall define the boundaries
    /// of the physical medium on which the page shall be displayed or printed.
    pub media_box: Option<Rectangle>,
    /// A rectangle, expressed in default user space units, that shall define the visible
    /// region of default user space. When the page is displayed or printed, its contents
    /// shall be clipped (cropped) to this rectangle
    pub crop_box: Option<Rectangle>,
    /// The number of degrees by which the page shall be rotated clockwise
    /// when displayed or printed. The value shall be a multiple of 90.
    /// Default value: 0.
    pub rotate: Option<RotationAngle>,
}

impl InheritablePageProperties {
    pub fn merge_with_parent(&mut self, parent_props: &InheritablePageProperties) {
        if let Some(media_box) = parent_props.media_box {
            self.media_box.get_or_insert(media_box);
        }
        if let Some(crop_box) = parent_props.crop_box {
            self.crop_box.get_or_insert(crop_box);
        }
        if let Some(rotate) = parent_props.rotate {
            self.rotate.get_or_insert(rotate);
        }
    }
}

#[derive(Debug, PartialEq, Clone, FromRawDict)]
pub struct PageProperties {
    /// A positive number that shall give the size of default user space units, in multiples
    /// of 1 ⁄ 72 inch. The range of supported values shall be implementation-dependent.
    ///
    /// Default value: 1.0 (user space unit is 1 ⁄ 72 inch).
    #[livre(default)]
    pub user_unit: f32,
}

#[derive(Debug, FromRawDict, Clone, PartialEq)]
struct PageTreeNode {
    #[livre(flatten)]
    props: InheritablePageProperties,
    #[livre(from = MaybeArray<Reference<PageElement>>)]
    kids: Vec<Reference<PageElement>>,
}

impl PageTreeNode {
    fn list_pages<B>(self, builder: &B) -> PResult<Vec<Page>>
    where
        B: Builder,
    {
        let Self { props, kids, .. } = self;

        let mut result = Vec::new();

        for kid in kids {
            let mut element = builder.build_reference(kid)?;
            element.merge_props(&props);

            match element {
                PageElement::Page(p) => result.push(p),
                PageElement::Node(n) => result.extend(n.list_pages(builder)?),
            }
        }

        Ok(result)
    }
}

#[derive(Debug, FromRawDict, Clone, PartialEq)]
pub struct Page {
    #[livre(flatten)]
    pub inheritable_props: InheritablePageProperties,
    #[livre(flatten)]
    pub props: PageProperties,
    #[livre(default, from = MaybeArray<Reference<Stream<()>>>)]
    pub contents: Vec<Reference<Stream<()>>>,
}

impl Page {
    pub fn build_content<B>(&self, builder: &B) -> PResult<Vec<u8>>
    where
        B: Builder,
    {
        let contents: PResult<Vec<Vec<u8>>> = self
            .contents
            .iter()
            .copied()
            .map(|reference| {
                let stream = builder.build_reference(reference)?;
                Ok::<Vec<u8>, ErrMode<ContextError>>(stream.content)
            })
            .collect();

        let content = contents?.into_iter().flatten().collect();
        Ok(content)
    }
}

/// Element from the page tree node.
#[derive(Debug, Clone, PartialEq)]
enum PageElement {
    Page(Page),
    Node(PageTreeNode),
}

impl PageElement {
    fn merge_props(&mut self, props: &InheritablePageProperties) {
        match self {
            Self::Page(page) => page.inheritable_props.merge_with_parent(props),
            Self::Node(node) => node.props.merge_with_parent(props),
        }
    }
}

impl FromRawDict<'_> for PageElement {
    fn from_raw_dict(dict: &mut RawDict<'_>) -> PResult<Self> {
        let Name(page_type) = dict
            .pop_and_extract(&"Type".into())
            .ok_or(ErrMode::Cut(ContextError::new()))??;

        let res = match page_type.as_slice() {
            b"Page" => Self::Page(Page::from_raw_dict(dict)?),
            b"Pages" => Self::Node(PageTreeNode::from_raw_dict(dict)?),
            _ => return Err(ErrMode::Cut(ContextError::new())),
        };

        Ok(res)
    }
}

impl Build for PageElement {
    fn build<B>(input: &mut &BStr, _builder: &B) -> PResult<Self>
    where
        B: Builder,
    {
        extract(input)
    }
}

/// An abstraction over PDF pages. This type does not actually reflect any PDF object.
/// Rather, it is constructed from the PDF page tree to obtain a "linearised" list of pages.
#[derive(Debug, PartialEq, Clone)]
pub struct Pages(pub Vec<Page>);

impl Build for Pages {
    fn build<B>(input: &mut &BStr, builder: &B) -> PResult<Self>
    where
        B: Builder,
    {
        let top_level: PageTreeNode = extract(input)?;
        let pages = top_level.list_pages(builder)?;
        Ok(Self(pages))
    }
}

impl Pages {
    pub fn iter(&self) -> impl Iterator<Item = &Page> {
        self.0.iter()
    }
}

#[cfg(test)]
mod tests {

    use indoc::indoc;
    use rstest::rstest;

    use crate::extraction::Extract;

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
        Rectangle::from((0.0, 0.0, 612., 792.))
    )]
    #[case(
        indoc! {b"
            <<
                /Type/Page
                /Parent 2 0 R
                /Resources<<
                    /XObject<<
                        /Image5 5 0 R
                        /Image18 18 0 R
                    >>
                    /ExtGState<<
                        /GS6 6 0 R
                        /GS9 9 0 R
                    >>
                    /Font<<
                        /F1 7 0 R
                        /F2 10 0 R
                        /F3 12 0 R
                        /F4 14 0 R
                        /F5 16 0 R
                        /F6 19 0 R
                        /F7 24 0 R
                        /F8 29 0 R
                        /F9 34 0 R
                    >>
                    /ProcSet[/PDF/Text/ImageB/ImageC/ImageI] 
                >>
                /MediaBox[ 0 0 595.32 841.92] 
                /Contents 4 0 R
                /Group<<
                    /Type/Group
                    /S/Transparency
                    /CS/DeviceRGB
                >>
                /Tabs/S
                /StructParents 0
            >>
        "},
        Rectangle::from((0.0, 0.0, 595.32, 841.92))
    )]
    #[case(
        indoc!{b"
            <<
                /Type/Page
                /Parent 2 0 R
                /Resources<<
                    /XObject<</Image5 5 0 R/Image18 18 0 R>>
                    /ExtGState<</GS6 6 0 R/GS9 9 0 R>>
                    /Font<</F1 7 0 R/F2 10 0 R/F3 12 0 R/F4 14 0 R/F5 16 0 R/F6 19 0 R/F7 24 0 R/F8 29 0 R/F9 34 0 R>>
                    /ProcSet[/PDF/Text/ImageB/ImageC/ImageI]
                >>
                /MediaBox[ 0 0 595.32 841.92] 
                /Contents 4 0 R
                /Group<</Type/Group/S/Transparency/CS/DeviceRGB>>
                /Tabs/S
                /StructParents 0
            >>
        "},
        Rectangle::from((0.0, 0.0, 595.32, 841.92))
    )]
    fn page(#[case] input: &[u8], #[case] expected: Rectangle) {
        let page = Page::extract(&mut input.as_ref()).unwrap();
        let InheritablePageProperties { media_box, .. } = page.inheritable_props;
        assert_eq!(media_box, Some(expected));
    }
}
