use std::{
    fs::File,
    io::{prelude::*, BufReader},
};

use livre::{
    fonts::FontBehavior,
    objects::Bytes,
    structure::{Build, BuiltPage, PageElement, PageNode},
};
use livre::{parsers::Extract, text::TextObjectIterator};
use livre::{
    structure::{Document, Page},
    text::TextObject,
};
use serde::Serialize;

fn parse_page_kids(node: &PageNode, doc: &Document) -> Vec<BuiltPage> {
    let mut pages = Vec::new();

    for &kid in &node.kids {
        match doc.parse_referenced(kid) {
            PageElement::Page(mut page) => {
                page.props.merge_with_parent(&node.props);
                pages.push(Page::from(page).build(doc))
            }
            PageElement::Pages(mut new_node) => {
                new_node.props.merge_with_parent(&node.props);
                pages.extend(parse_page_kids(&new_node, doc))
            }
        }
    }

    pages
}

#[derive(Debug, Serialize)]
struct Element {
    text: char,
    font: String,
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
    group: usize,
    size: f32,
}

impl Element {
    fn new(
        text: char,
        font: String,
        min_x: f32,
        max_x: f32,
        min_y: f32,
        max_y: f32,
        group: usize,
        size: f32,
    ) -> Self {
        Self {
            text,
            font,
            min_x,
            max_x,
            min_y,
            max_y,
            group,
            size,
        }
    }
}

fn main() {
    let file = File::open("/Users/basile/Downloads/document.pdf").unwrap();
    // let file = File::open("resource/ISO_32000-2-2020_sponsored.pdf").unwrap();
    let mut reader = BufReader::new(file);

    let mut input: Vec<u8> = Vec::new();
    reader.read_to_end(&mut input).ok();

    let (_, doc) = Document::extract(&input)
        .map_err(|e| e.map_input(Bytes::from))
        .unwrap();

    let root = doc.parse_referenced(doc.root);
    // println!("{root:?}");

    let pages: PageNode = doc.parse_referenced(root.pages);

    let mut pages = parse_page_kids(&pages, &doc);

    let page = pages.pop().unwrap();

    let file = File::create("test.csv").unwrap();
    let mut wtr = csv::Writer::from_writer(file);

    let from = TextObjectIterator::from(&page);

    for (i, TextObject { content, mut state }) in from.enumerate() {
        println!();
        println!("# {i}: {:?}", &state.font);
        for operator in content {
            println!("{operator:?}");
            state.apply(operator);
        }

        for element in state.elements {
            let element = Element::new(
                element.char,
                state.font.name().to_string(),
                element.bounding_box.lower_left.x,
                element.bounding_box.upper_right.x,
                element.bounding_box.lower_left.y,
                element.bounding_box.upper_right.y,
                i,
                state.size,
            );
            wtr.serialize(element).unwrap();
        }
    }
    dbg!(Bytes::from(page.content.as_slice()));
}
