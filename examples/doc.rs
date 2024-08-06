use std::{
    fs::File,
    io::{prelude::*, BufReader},
};

use livre::objects::{Reference, Stream};
use livre::parsers::{extract, DbgStr, Extract};
use livre::serde::extract_deserialize;
use livre::structure::{Catalogue, PageElement, PageLeaf, PageNode};
use livre::{document::DocumentBuilder, structure::Page};
use serde::Deserialize;

fn parse_page_kids(node: &PageNode, doc: &DocumentBuilder) -> Vec<Page> {
    let mut pages = Vec::new();

    for &kid in &node.kids {
        match doc.parse_referenced(kid) {
            PageElement::Page(mut page) => {
                page.props.merge_with_parent(&node.props);
                pages.push(page.into())
            }
            PageElement::Pages(mut new_node) => {
                new_node.props.merge_with_parent(&node.props);
                pages.extend(parse_page_kids(&new_node, doc))
            }
        }
    }

    pages
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Font {
    subtype: String,
    base_font: String,
    font_descriptor: Option<Reference>,
}

impl Extract<'_> for Font {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        extract_deserialize(input)
    }
}

fn get_decoded(doc: &DocumentBuilder, reference: Reference) -> String {
    let (_, DbgStr(decoded)) = extract(doc.get_referenced_bytes(reference).unwrap()).unwrap();
    decoded.to_string()
}

fn main() {
    let file = File::open("tests/letter.pdf").unwrap();
    // let file = File::open("resource/ISO_32000-2-2020_sponsored.pdf").unwrap();
    let mut reader = BufReader::new(file);

    let mut input: Vec<u8> = Vec::new();
    reader.read_to_end(&mut input).ok();

    let (_, doc) = DocumentBuilder::extract(&input).unwrap();
    for xref in &doc.crossrefs {
        println!("{:?}", xref);
    }

    let root: Catalogue = doc.parse_referenced(doc.root);
    println!("{root:?}");

    let pages: PageNode = doc.parse_referenced(root.pages);
    println!("P: {pages:?}");

    let pages = parse_page_kids(&pages, &doc);

    for page in &pages {
        println!("{page:?}");
        // let &reference = page.contents.0.first().unwrap();
        for (k, &reference) in page.resources.font.iter() {
            let decoded_font = get_decoded(&doc, reference);

            let font: Font = doc.parse_referenced(reference);

            println!("\nFont {k}:\n{decoded_font}{font:?}");
            // if let Some(decoded_descriptor) = font.font_descriptor.map(|fd| get_decoded(&doc, fd)) {
            //     println!("{decoded_descriptor}");
            // }
            // let &reference = page.resources;
        }
        // let ContentStream(content) = doc.parse_referenced(reference, &input);
        // let (_, DbgStr(decoded)) = extract(&content).unwrap();
        // println!("{}", &decoded);
    }

    // let page_raw = doc.get_referenced_bytes(pages.kids[0]).unwrap();
    // // let (_, IntoString(page)) = extract(page_raw).unwrap();
    // let (_, page) = extract::<PageLeaf>(page_raw).unwrap();

    // let content_raw = doc.get_referenced_bytes(page.contents.0[0]).unwrap();
    // // let (_, IntoString(content)) = extract(content_raw).unwrap();
    // let (_, content) = extract::<Stream<'_, NoOp>>(content_raw).unwrap();
    // let decoded = content.decode().unwrap();
    // let decoded = String::from_utf8_lossy(&decoded);
    // println!("{decoded}");

    // for line in decoded
    //     .split('\n')
    //     .filter(|t| t.to_lowercase().contains("tj"))
    // {
    //     println!("{line:?}");
    // }

    // let font: Font = doc.parse_referenced(TypedReference::new(object, generation))

    // println!("{decoded}");
    // let (_, DbgStr(font)) = extract(
    //     doc.get_referenced_bytes(Reference::new(20, 0), &input)
    //         .unwrap(),
    // )
    // .unwrap();

    // println!("F4 Widths:\n{font}");

    // let (
    //     _,
    //     Stream {
    //         decoded,
    //         structured: (),
    //     },
    // ) = extract(
    //     doc.get_referenced_bytes(Reference::new(183, 0), &input)
    //         .unwrap(),
    // )
    // .unwrap();

    // let (_, DbgStr(decoded)) = extract(&decoded).unwrap();

    // println!("ToUnicode:\n{decoded}");
}
