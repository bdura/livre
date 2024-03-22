use livre_extraction::{Extract, FromDictRef, MaybeArray, NoOp, Reference, TypedReference};
use livre_objects::Stream;

use super::{PageProperties, Variant};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ContentStream(pub Vec<u8>);

impl Extract<'_> for ContentStream {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        let (_, stream) = Stream::<'_, NoOp>::extract(input)?;
        let decoded = stream.decode().unwrap().into();
        Ok((input, Self(decoded)))
    }
}

#[derive(Debug, PartialEq, Clone, FromDictRef, Extract)]
pub struct PageLeaf {
    #[livre(rename = "Type")]
    pub variant: Variant,
    pub parent: Reference,
    // #[livre(rename = "LastModified")]
    // pub last_modified: DateTime
    // pub resources
    // pub crop_box: Option<Rectangle>,
    // pub bleed_box: Option<Rectangle>,
    // pub trim_box: Option<Rectangle>,
    // pub art_box: Option<Rectangle>,
    // pub box_color_info
    #[livre(flatten)]
    pub props: PageProperties,
    #[livre(default)]
    pub contents: MaybeArray<TypedReference<ContentStream>>,
    // pub rotate: Option<u8>,
    // pub group: Option<...>
    // pub thumb: Option<...>
    // pub b: Option<...>
    // pub dur: Option<...>
    // pub trans: Option<...>
    // pub annots: Option<Vec<...annotation dict...>>
    // and more!
    pub user_unit: Option<f32>,
}

#[cfg(test)]
mod tests {

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
        Reference::new(4, 0)
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
        Reference::new(2, 0)
    )]
    fn page(#[case] input: &[u8], #[case] parent: Reference) {
        let (_, page) = PageLeaf::extract(input).unwrap();
        assert_eq!(page.parent, parent);
    }
}
