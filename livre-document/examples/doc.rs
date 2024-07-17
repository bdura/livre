use std::{
    fs::File,
    io::{prelude::*, BufReader},
};

use livre_document::Document;
use livre_extraction::{extract, DbgStr, Extract, Name, NoOp};
use livre_objects::{Reference, Stream};
use livre_structure::{Catalogue, PageElement, PageLeaf, PageNode};

fn parse_page_kids(node: &PageNode, doc: &Document, input: &[u8]) -> Vec<PageLeaf> {
    let mut pages = Vec::new();

    for &kid in &node.kids {
        let element = doc.parse_referenced(kid, input);

        match element {
            PageElement::Leaf(mut page) => {
                page.props.merge_with_parent(&node.props);
                pages.push(page)
            }
            PageElement::Node(mut new_node) => {
                new_node.props.merge_with_parent(&node.props);
                pages.extend(parse_page_kids(&new_node, doc, input))
            }
        }
    }

    pages
}

#[derive(Extract, Debug)]
struct Font {
    #[livre(from = Name)]
    subtype: String,
    #[livre(from = Name)]
    base_font: String,
    font_descriptor: Option<Reference>,
}

fn get_decoded(input: &[u8], doc: &Document, reference: Reference) -> String {
    let (_, DbgStr(decoded)) =
        extract(doc.get_referenced_bytes(reference, &input).unwrap()).unwrap();
    decoded.to_string()
}

fn main() {
    let file = File::open("tests/letter.pdf").unwrap();
    // let file = File::open("resource/ISO_32000-2-2020_sponsored.pdf").unwrap();
    let mut reader = BufReader::new(file);

    let mut input: Vec<u8> = Vec::new();
    reader.read_to_end(&mut input).ok();

    let (_, doc) = Document::extract(&input).unwrap();
    for xref in &doc.crossrefs {
        println!("{:?}", xref);
    }

    let root: Catalogue = doc.parse_referenced(doc.root, &input);
    println!("{root:?}");

    let pages: PageNode = doc.parse_referenced(root.pages, &input);
    println!("P: {pages:?}");

    let pages = parse_page_kids(&pages, &doc, &input);

    for page in &pages {
        println!("{page:?}");
        // let &reference = page.contents.0.first().unwrap();
        for (k, &reference) in page.resources.font.iter() {
            let decoded_font = get_decoded(&input, &doc, reference);

            let font: Font = doc.parse_referenced(reference.into(), &input);

            println!("\nFont {k}:\n{decoded_font}{font:?}");
            if let Some(decoded_descriptor) =
                font.font_descriptor.map(|fd| get_decoded(&input, &doc, fd))
            {
                println!("{decoded_descriptor}");
            }
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
    let (_, DbgStr(font)) = extract(
        doc.get_referenced_bytes(Reference::new(20, 0), &input)
            .unwrap(),
    )
    .unwrap();

    println!("F4 Widths:\n{font}");

    let (_, stream) = Stream::<NoOp>::extract(
        doc.get_referenced_bytes(Reference::new(183, 0), &input)
            .unwrap(),
    )
    .unwrap();

    let decoded = stream.decode().unwrap();
    let (_, DbgStr(decoded)) = extract(&decoded).unwrap();

    println!("ToUnicode:\n{decoded}");
}
