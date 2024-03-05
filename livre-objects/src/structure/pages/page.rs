use livre_extraction::{Extract, FromDictRef, MaybeArray, Reference};
use livre_structures::Rectangle;
use nom::{branch::alt, bytes::complete::tag, combinator::map};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Variant {
    Page,
    Template,
}

impl Extract<'_> for Variant {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        alt((
            map(tag("/Page"), |_| Variant::Page),
            map(tag("/Template"), |_| Variant::Template),
        ))(input)
    }
}

#[derive(Debug, PartialEq, Clone, FromDictRef, Extract)]
pub struct Page {
    #[livre(rename = "Type")]
    pub variant: Variant,
    pub parent: Reference,
    // #[livre(rename = "LastModified")]
    // pub last_modified: DateTime
    // pub resources
    #[livre(rename = "MediaBox")]
    pub media_box: Option<Rectangle>,
    // #[livre(rename = "CropBox")]
    // pub crop_box: Option<Rectangle>,
    // #[livre(rename = "BleedBox")]
    // pub bleed_box: Option<Rectangle>,
    // #[livre(rename = "TrimBox")]
    // pub trim_box: Option<Rectangle>,
    // #[livre(rename = "ArtBox")]
    // pub art_box: Option<Rectangle>,
    // pub box_color_info
    pub contents: Option<MaybeArray<Reference>>,
    pub rotate: Option<u8>,
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

    use super::*;

    #[test]
    fn page() {
        let input = indoc! {b"
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
        "};
        println!("{}", String::from_utf8_lossy(input));
        let (_, page) = Page::extract(input).unwrap();
        assert_eq!(page.parent, Reference::new(4, 0));
    }
}
