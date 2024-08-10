use std::{
    fs::File,
    io::{prelude::*, BufReader},
};

use livre::{
    fonts::FontBehavior,
    structure::{PageElement, PageNode},
};
use livre::{parsers::Extract, structure::ContentStream};
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

    let root = doc.parse_referenced(doc.root);
    println!("{root:?}");

    let pages: PageNode = doc.parse_referenced(root.pages);

    let mut pages = parse_page_kids(&pages, &doc);

    let mut page = pages.pop().unwrap();

    let content_ref = page.contents.pop().unwrap();
    let ContentStream(content) = doc.parse_referenced(content_ref);

    let decoded = String::from_utf8_lossy(&content);

    let mut filter = true;

    let iter = decoded
        .split('\n')
        .map(|t| t.strip_suffix('\r').unwrap_or(t));

    // let iter = content
    //     .split(|&b| b == b'\n')
    //     .map(|b| b.strip_suffix(b"\r").unwrap_or(b));

    for line in iter.clone() {
        if filter && line == "BT" {
            filter = false;
        }

        if !filter {
            println!("{line}");
        }

        if !filter && line == "ET" {
            filter = true;
            println!();
        }
    }

    let f5 = page.get_font("F5".to_string(), &doc);

    println!();
    println!("F5 -> {f5:?}");
    println!();
    println!("{}", f5.width(0o340));
    println!("{}", f5.width(b' ' as usize));
    println!("{}", f5.width(b'i' as usize));
    println!("{}", f5.width(b'A' as usize));
    println!("{}", f5.width(b'-' as usize));

    for line in content
        .split(|&b| b == b'\n')
        .map(|b| b.strip_suffix(b"\r").unwrap_or(b))
        .filter(|b| b.ends_with(b"TJ"))
        .filter(|b| b.contains(&0o340))
    {
        println!("{}", String::from_utf8_lossy(line));
    }
}
