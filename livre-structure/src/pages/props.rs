use livre_data::Rectangle;
use livre_extraction::{Extract, FromDictRef};

#[derive(Debug, PartialEq, Clone, FromDictRef, Extract)]
pub struct PageProperties {
    // pub resource: Option<>
    /// A rectangle expressed in default user space units,
    /// that shall define the boundaries of the physical medium
    /// on which the page shall be displayed or printed
    pub media_box: Option<Rectangle>,
    /// A rectangle, expressed in default user space units,
    /// that shall define the visible region of default user space.
    /// When the page is displayed or printed, its contents shall be
    /// clipped (cropped) to this rectangle
    pub crop_box: Option<Rectangle>,
    /// The number of degrees by which the page shall be rotated clockwise
    /// when displayed or printed. The value shall be a multiple of 90.
    /// Default value: 0.
    pub rotate: Option<i16>,
}

#[cfg(test)]
mod tests {

    use indoc::indoc;
    use livre_extraction::RawDict;
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
        let (_, mut raw_dict) = RawDict::extract(input).unwrap();
        let (_, props) = PageProperties::extract(input).unwrap();
        assert_eq!(props, PageProperties::from_dict_ref(&mut raw_dict).unwrap());
        assert_eq!(props.media_box, Some(expected));
    }
}
