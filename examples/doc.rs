use std::{
    fs::File,
    io::{prelude::*, BufReader},
};

use livre::parsers::Extract;
use livre::structure::{Build, Catalogue, PageElement, PageNode};
use livre::{
    fonts::FontTransient,
    objects::{Object, Reference},
};
use livre::{structure::Document, structure::Page};

fn parse_page_kids(node: &PageNode, doc: &Document) -> Vec<Page> {
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

fn main() {
    let file = File::open("tests/letter.pdf").unwrap();
    // let file = File::open("resource/ISO_32000-2-2020_sponsored.pdf").unwrap();
    let mut reader = BufReader::new(file);

    let mut input: Vec<u8> = Vec::new();
    reader.read_to_end(&mut input).ok();

    let (_, doc) = Document::extract(&input).unwrap();
    for xref in &doc.crossrefs {
        println!("{:?}", xref);

        // let object: Object = doc.parse_referenced(*xref.0);
        // println!("{object:?}");
        // println!();

        // if let RefLocation::Uncompressed(loc) = *xref.1 {
        //     let bytes = &input[loc..];

        //     println!(
        //         "Obj: {:?}",
        //         String::from_utf8_lossy(&bytes[..bytes.len().min(100)])
        //     );
        // }
    }

    let root = doc.parse_referenced(doc.root);
    println!("{root:?}");

    let pages: PageNode = doc.parse_referenced(root.pages);
    // println!("P: {pages:?}");

    let mut pages = parse_page_kids(&pages, &doc);

    // for (i, page) in pages.iter().enumerate() {
    //     println!("# {i}\n{page:#?}");

    //     let content = doc.parse_referenced(*page.contents.first().unwrap());

    //     println!("CONTENT:\n{content:#?}");
    // }

    let mut page = pages.pop().unwrap();

    let fonts = &page.resources.font;

    for (key, reference) in fonts.clone() {
        let font = page.get_font(key.clone(), &doc);
        println!();
        println!("{key} -> {font:?}");

        //let r: Reference = (*reference).into();
        //let object: Object = doc.parse_referenced(r);
        //// println!("{object:#?}");
        //
        //let font: FontTransient = doc.parse_referenced(r);
        //println!();
        //println!("{r:?}\n{font:?}\n{object:?}");
        //
        //if let FontTransient::Type0(type0) = font {
        //    let type0 = type0.build(&doc);
        //    // let descendant = descendant_fonts
        //    //     .get_or_instantiate(&doc)
        //    //     .first_mut()
        //    //     .unwrap()
        //    //     .get_or_instantiate(&doc);
        //    println!("Desc: {:?}", type0.descendant_font);
        //}
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

    // let content = doc.parse_referenced(*pages[0].contents.first().unwrap());

    // for line in String::from_utf8_lossy(&content.0)
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
