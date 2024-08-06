use crate::data::Rectangle;
use serde::Deserialize;

use super::resources::Resources;

/// Inheritable page properties. All values shall be inherited as-is,
/// without merging.
///
/// Thus for instance the [Resources] dictionary for a page shall be
/// found by searching the [PageLeaf](super::PageLeaf) object and then
/// each [PageNode](super::PageNode) object encountered by following
/// `Parent`` links up the pages tree from the Page towards the Catalog object.
/// When the first Resources dictionary is found the search shall be stopped
/// and that Resources dictionary shall be used in its entirety.
#[derive(Debug, PartialEq, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct InheritablePageProperties {
    /// A dictionary containing any resources required
    /// by the page contents
    #[serde(default)]
    pub resources: Option<Resources>,
    /// A rectangle expressed in default user space units,
    /// that shall define the boundaries of the physical medium
    /// on which the page shall be displayed or printed
    #[serde(default)]
    pub media_box: Option<Rectangle>,
    /// A rectangle, expressed in default user space units,
    /// that shall define the visible region of default user space.
    /// When the page is displayed or printed, its contents shall be
    /// clipped (cropped) to this rectangle
    #[serde(default)]
    pub crop_box: Option<Rectangle>,
    /// The number of degrees by which the page shall be rotated clockwise
    /// when displayed or printed. The value shall be a multiple of 90.
    /// Default value: 0.
    #[serde(default)]
    pub rotate: Option<i16>,
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

#[cfg(test)]
mod tests {

    use crate::serde::extract_deserialize;
    use indoc::indoc;
    use rstest::rstest;

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
        Rectangle::from_ll_ur(0.0, 0.0, 612., 792.)
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
        Rectangle::from_ll_ur(0.0, 0.0, 595.32, 841.92)
    )]
    fn page(#[case] input: &[u8], #[case] expected: Rectangle) {
        let (_, InheritablePageProperties { media_box, .. }) = extract_deserialize(input).unwrap();
        assert_eq!(media_box, Some(expected));
    }
}
