use std::{
    fs::File,
    io::{prelude::*, BufReader},
};

use livre::{
    fonts::FontBehavior,
    structure::{Build, BuiltPage, PageElement, PageNode},
};
use livre::{parsers::Extract, text::TextObjectIterator};
use livre::{
    structure::{Document, Page},
    text::TextObject,
};

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

fn main() {
    let file = File::open("tests/letter.pdf").unwrap();
    // let file = File::open("resource/ISO_32000-2-2020_sponsored.pdf").unwrap();
    let mut reader = BufReader::new(file);

    let mut input: Vec<u8> = Vec::new();
    reader.read_to_end(&mut input).ok();

    let (_, doc) = Document::extract(&input).unwrap();

    let root = doc.parse_referenced(doc.root);
    // println!("{root:?}");

    let pages: PageNode = doc.parse_referenced(root.pages);

    let mut pages = parse_page_kids(&pages, &doc);

    let page = pages.pop().unwrap();

    let decoded = String::from_utf8_lossy(&page.content);

    let mut filter = true;

    let iter = decoded
        .split('\n')
        .map(|t| t.strip_suffix('\r').unwrap_or(t));

    for line in iter {
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

    // for (i, TextObject { content, mut state }) in TextObjectIterator::from(&page).enumerate() {
    //     println!();
    //     println!("# {i}");
    //     for operator in content {
    //         println!("- OP {operator:?}");
    //         state.apply(operator);
    //     }
    // }

    let font_name = "F7";
    println!("{font_name} -> {:?}", page.fonts.get(font_name).unwrap());

    // println!();
    // println!("F5 -> {f5:?}");

    // for line in page
    //     .content
    //     .split(|&b| b == b'\n')
    //     .map(|b| b.strip_suffix(b"\r").unwrap_or(b))
    //     .filter(|b| b.ends_with(b"TJ"))
    //     .filter(|b| b.contains(&0o340))
    // {
    //     println!("{}", String::from_utf8_lossy(line));
    // }

    print_page_elements(&page);
}

fn print_page_elements(page: &BuiltPage) {
    let mut file = File::create("./letter.csv").unwrap();

    writeln!(file, "group,text,font,llx,lly,urx,ury").unwrap();

    for (i, TextObject { content, mut state }) in TextObjectIterator::from(page).enumerate() {
        for operator in content {
            state.apply(operator);
        }

        for element in state.elements {
            writeln!(
                file,
                r#"{i},"{}","{}",{},{},{},{}"#,
                element.char,
                state.font.name(),
                element.bounding_box.lower_left.x,
                element.bounding_box.lower_left.y,
                element.bounding_box.upper_right.x,
                element.bounding_box.upper_right.y
            )
            .unwrap();
        }
    }
}
