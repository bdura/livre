use std::collections::HashMap;
use std::fmt::Debug;

use crate::data::Rectangle;
use crate::fonts::Font;
use crate::objects::Stream;
use crate::parsers::{extract, Extract, TypedReference};
use crate::serde::MaybeArray;
use crate::structure::{Build, Document};
use serde::Deserialize;

use super::resources::Resources;
use super::InheritablePageProperties;
use super::PageNode;

#[derive(Default, Clone, PartialEq, Eq)]
pub struct ContentStream(pub Vec<u8>);

impl Debug for ContentStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ContentStream")
            .field(&String::from_utf8_lossy(&self.0))
            .finish()
    }
}

impl Extract<'_> for ContentStream {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        let (
            _,
            Stream {
                decoded,
                structured: (),
            },
        ) = extract(input)?;
        Ok((input, Self(decoded.0)))
    }
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PageLeaf {
    pub parent: TypedReference<PageNode>,
    // pub crop_box: Option<Rectangle>,
    // pub bleed_box: Option<Rectangle>,
    // pub trim_box: Option<Rectangle>,
    // pub art_box: Option<Rectangle>,
    // pub box_color_info
    #[serde(flatten)]
    pub props: InheritablePageProperties,
    #[serde(default)]
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

#[derive(Debug, PartialEq, Clone)]
pub struct Page {
    pub resources: Resources,
    pub media_box: Rectangle,
    pub crop_box: Rectangle,
    pub rotate: i16,
    pub contents: Vec<TypedReference<ContentStream>>,
    // pub rotate: Option<u8>,
    // pub group: Option<...>
    // pub thumb: Option<...>
    // pub b: Option<...>
    // pub dur: Option<...>
    // pub trans: Option<...>
    // pub annots: Option<Vec<...annotation dict...>>
    // and more!
    pub user_unit: f32,
    /// Cached fonts
    pub fonts: HashMap<String, Font>,
}

impl From<PageLeaf> for Page {
    fn from(
        PageLeaf {
            props,
            contents: MaybeArray(contents),
            user_unit,
            ..
        }: PageLeaf,
    ) -> Self {
        let resources = props.resources.expect("Required per the specs");
        let media_box = props.media_box.expect("Required per the specs");
        let crop_box = props.crop_box.unwrap_or(media_box);
        let rotate = props.rotate.unwrap_or(0);

        Self {
            resources,
            media_box,
            crop_box,
            rotate,
            contents,
            user_unit: user_unit.unwrap_or(1.0),
            fonts: HashMap::new(),
        }
    }
}

impl Page {
    pub fn get_font<'a>(&'a mut self, name: String, doc: &Document) -> &'a Font {
        let &reference = self.resources.font.get(&name).unwrap();
        self.fonts
            .entry(name.into())
            .or_insert_with(|| doc.parse_referenced(reference).build(doc))
    }
}

#[cfg(test)]
mod tests {

    use crate::objects::{Bytes, Reference};
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
    fn page(#[case] input: &[u8], #[case] expected: Reference) {
        let (_, PageLeaf { parent, .. }) = extract_deserialize(input)
            .map_err(|e| e.map_input(Bytes::from))
            .unwrap();
        assert_eq!(expected, parent.into());
    }
}
