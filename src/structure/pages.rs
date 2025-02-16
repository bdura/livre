use std::collections::HashMap;

use winnow::{
    error::{ContextError, ErrMode},
    BStr, PResult, Parser,
};

use crate::{
    extraction::{
        extract, Date, Extract, FromRawDict, Id, Map, MaybeArray, Name, OptRef, RawDict, Rectangle,
        Reference, Stream, Todo,
    },
    follow_refs::{Build, Builder},
};

/// Page resources.
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

/// Page properties that cannot be inherited.
#[derive(Debug, PartialEq, Clone, FromRawDict)]
pub struct IndividualPageProperties {
    /// The date and time when the page's contents were most recently modified.
    pub last_modified: Option<Date>,
    /// A rectangle, expressed in default user space units, that shall define the region
    /// to which the contents of the page shall be clipped when output in a production
    /// environment.
    ///
    /// Default value: the value of [`CropBox`](InheritablePageProperties::crop_box).
    pub bleed_box: Option<Rectangle>,
    /// A rectangle, expressed in default user space units, that shall define the intended
    /// dimensions of the finished page after trimming.
    ///
    /// Default value: the value of [`CropBox`](InheritablePageProperties::crop_box).
    pub trim_box: Option<Rectangle>,
    /// A rectangle, expressed in default user space units, that shall define the extent
    /// of the page’s meaningful content (including potential white-space) as intended
    /// by the page’s creator.
    ///
    /// Default value: the value of [`CropBox`](InheritablePageProperties::crop_box).
    pub art_box: Option<Rectangle>,
    /// A box colour information dictionary that shall specify the colours and other
    /// visual characteristics that should be used in displaying guidelines on the screen
    /// for the various page boundaries.
    ///
    /// If this entry is absent, the application shall use its own current default settings.
    pub box_color_info: Option<Map<Todo>>,
    /// A group attributes dictionary that shall specify the attributes of the page’s
    /// page group for use in the transparent imaging model.
    pub group: Option<Map<Todo>>,
    /// A stream object that shall define the page’s thumbnail image.
    pub thumb: Option<Reference<Todo>>,
    /// An array that shall contain indirect references to all article beads appearing
    /// on the page. The beads shall be listed in the array in natural reading order.
    ///
    /// Template pages do no have a `B` key. This field is "recommended if the page contains
    /// article beads".
    #[livre(rename = "B")]
    pub beads: Option<Vec<Reference<Todo>>>,
    /// The page’s display duration (also called its advance timing): the maximum
    /// length of time, in seconds, that the page shall be displayed during presentations
    /// before the viewer application shall automatically advance to the next page.
    /// By default, the viewer shall not advance automatically.
    pub dur: Option<f64>,
    /// A transition dictionary describing the transition effect that shall be used when
    /// displaying the page during presentations.
    pub trans: Option<Map<Todo>>,
    /// An array of annotation dictionaries that shall contain indirect references to all
    /// annotations associated with the page.
    pub annots: Option<Vec<Reference<Todo>>>,
    /// A metadata stream that shall contain metadata for the page.
    #[livre(rename = "AA")]
    pub additional_annotations: Option<Map<Reference<Todo>>>,
    /// A metadata stream that shall contain metadata for the page.
    pub metadata: Option<Reference<Stream<()>>>,
    /// A page-piece dictionary associated with the page.
    pub piece_info: Option<Map<Todo>>,
    /// The integer key of the page’s entry in the structural parent tree.
    pub struct_parents: Option<Todo>,
    /// The digital identifier of the page’s parent Web Capture content set.
    #[livre(rename = "ID")]
    pub id: Option<Id>,
    /// The page’s preferred zoom (magnification) factor: the factor by which it shall
    /// be scaled to achieve the natural display magnification.
    #[livre(rename = "PZ")]
    pub preferred_zoom: Option<f64>,
    /// A separation dictionary that shall contain information needed to generate colour
    /// separations for the page.
    pub separation_info: Option<Map<Todo>>,
    /// A name specifying the tab order that shall be used for annotations on the page.
    ///
    /// Possible values: `R` (row order), `C` (column order) or `S` (structure order).
    pub tabs: Option<Name>,
    /// The name of the originating page object.
    ///
    /// Required if this page was created from a named page object.
    pub template_instantiated: Option<Name>,
    /// A navigation node dictionary that shall represent the first node on the page.
    pub pres_steps: Option<Map<Todo>>,
    /// An array of viewport dictionaries that shall specify rectangular regions of the page.
    #[livre(rename = "VP")]
    pub viewport_dictionaries: Option<Vec<Reference<()>>>,
    /// An array of one or more file specification dictionaries which denote the associated
    /// files for this page.
    #[livre(rename = "AF")]
    pub associated_files: Option<Vec<Reference<()>>>,
    /// An array of output intent dictionaries that shall specify the colour characteristics
    /// of output devices on which this page might be rendered
    pub output_indents: Option<Vec<Reference<Todo>>>,
    /// An indirect reference to the DPart dictionary whose range of pages includes this page
    /// object
    ///
    /// Required, if this page is within the range of a DPart, not permitted otherwise
    #[livre(rename = "DPart")]
    pub dpart: Option<Reference<()>>,
}

/// Inheritable page properties. Each attribute shall be inherited as-is, without merging.
///
/// Due to their inheritable nature, every field is wrapped in an [`Option`], whether
/// they are optional or not in the final properties dictionary.
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

/// Intermediate page nodes, and the type of the `Pages` element in the document's
/// [`Catalog`](super::Catalog).
///
/// From the PDF specification:
///
/// > The simplest structure can consist of a single page tree node that references
/// > all of the document’s page objects directly. However, to optimise application performance,
/// > a PDF writer can construct trees of a particular form, known as balanced trees.
///
/// In Livre, the page tree is immediately transformed into a vector of [`Page`]s for simplicity.
/// Hence this type, along with other low-level types, are made public for reference only.
#[derive(Debug, FromRawDict, Clone, PartialEq)]
pub struct PageTreeNode {
    /// Properties that can be passed down to `Kids` pages.
    #[livre(flatten)]
    props: InheritablePageProperties,
    /// Array of indirect references other pages.
    #[livre(from = MaybeArray<Reference<PageElement>>)]
    kids: Vec<Reference<PageElement>>,
}

impl Build for PageTreeNode {
    fn build<B>(input: &mut &BStr, _: &B) -> PResult<Self>
    where
        B: Builder,
    {
        extract(input)
    }
}

impl PageTreeNode {
    pub fn list_pages<B>(&self, builder: &B) -> PResult<Vec<Page>>
    where
        B: Builder,
    {
        let Self { props, kids, .. } = self;

        let mut result = Vec::new();

        for &kid in kids {
            let mut element = builder.build_reference(kid)?;
            element.merge_props(props);

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
    //#[livre(flatten)]
    //pub individual_props: IndividualPageProperties,
    /// From the specification:
    ///
    /// > A content stream that shall describe the contents of this page. If this entry is absent,
    /// > the page shall be empty. The value shall be either a single stream or an array
    /// > of streams. If the value is an array, the effect shall be as if all of the streams
    /// > in the array were concatenated with at least one white-space character added between
    /// > the streams’ data, in order, to form a single stream. PDF writers can create image
    /// > objects and other resources as they occur, even though they interrupt the content
    /// > stream.
    /// >
    /// > The division between streams may occur only at the boundaries between lexical tokens
    /// > but shall be unrelated to the page’s logical content or organisation. Applications
    /// > that consume or produce PDF files need not preserve the existing structure of the
    /// > Contents array.
    /// >
    /// > PDF writers shall not create a Contents array containing no elements.
    #[livre(default, from = MaybeArray<Reference<Stream<()>>>)]
    pub contents: Vec<Reference<Stream<()>>>,
    /// A positive number that shall give the size of default user space units, in multiples
    /// of 1 ⁄ 72 inch. The range of supported values shall be implementation-dependent.
    ///
    /// Default value: 1.0 (user space unit is 1 ⁄ 72 inch).
    #[livre(default)]
    pub user_unit: f32,
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
#[allow(
    clippy::large_enum_variant,
    reason = "This enumeration is meant to be transient."
)]
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
