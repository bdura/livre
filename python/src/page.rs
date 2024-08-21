use std::{
    fs::File,
    io::{prelude::*, BufReader},
};

use pyo3::prelude::*;

use livre::{
    data::Rectangle,
    structure::{Document, Page},
    text::TextObject,
};
use livre::{
    fonts::FontBehavior,
    structure::{Build, BuiltPage, PageElement, PageNode},
    text::{TextElement, TextState},
};
use livre::{parsers::Extract, text::TextObjectIterator};

#[derive(Debug)]
#[pyclass]
pub struct PdfElement {
    #[pyo3(get)]
    pub content: char,
    #[pyo3(get)]
    pub font_name: String,
    #[pyo3(get)]
    pub object_id: usize,
    #[pyo3(get)]
    pub size: f32,
    #[pyo3(get)]
    pub min_x: f32,
    #[pyo3(get)]
    pub max_x: f32,
    #[pyo3(get)]
    pub min_y: f32,
    #[pyo3(get)]
    pub max_y: f32,
}

impl PdfElement {
    fn new(element: &TextElement, state: &TextState, object_id: usize) -> Self {
        let TextElement {
            char,
            bounding_box:
                Rectangle {
                    lower_left,
                    upper_right,
                },
        } = element;
        let TextState { font, size, .. } = state;
        Self {
            object_id,
            content: *char,
            font_name: font.name().to_string(),
            size: *size,
            min_x: lower_left.x,
            max_x: upper_right.x,
            min_y: lower_left.y,
            max_y: upper_right.y,
        }
    }
}

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

pub fn read_page(path: &str, page: usize) -> Vec<PdfElement> {
    let file = File::open(path).unwrap();

    let mut reader = BufReader::new(file);
    let mut input: Vec<u8> = Vec::new();
    reader.read_to_end(&mut input).ok();

    let (_, doc) = Document::extract(&input).unwrap();

    let root = doc.parse_referenced(doc.root);
    let pages: PageNode = doc.parse_referenced(root.pages);
    let pages = parse_page_kids(&pages, &doc);

    if page >= pages.len() {
        panic!(
            "You requested page {}, but the document only contains {}",
            page + 1,
            pages.len()
        );
    }

    let page = &pages[page];

    export_page_elements(page)
}

fn export_page_elements(page: &BuiltPage) -> Vec<PdfElement> {
    let mut elements = Vec::new();

    for (i, TextObject { content, mut state }) in TextObjectIterator::from(page).enumerate() {
        // Apply every operators
        for operator in content {
            state.apply(operator);
        }

        // Extend the elements vector
        for element in &state.elements {
            elements.push(PdfElement::new(element, &state, i))
        }
    }

    elements
}
