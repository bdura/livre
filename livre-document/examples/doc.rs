use std::{
    fs::File,
    io::{prelude::*, BufReader},
};

use livre_document::Document;
use livre_extraction::{extract, Extract, NoOp};
use livre_objects::Stream;
use livre_structure::{Catalogue, PageLeaf, PageNode};

fn main() {
    let file = File::open("tests/letter.pdf").unwrap();
    let mut reader = BufReader::new(file);

    let mut buf: Vec<u8> = Vec::new();
    reader.read_to_end(&mut buf).ok();

    // reader.seek(std::io::SeekFrom::End(-100)).unwrap();

    let (_, doc) = Document::extract(&buf)
        // .map_err(|_| ExtractionError::ParsingError("test".to_string()))
        .unwrap();

    let root_raw = doc.get_referenced_bytes(doc.root).unwrap();
    let (_, root) = extract::<Catalogue>(root_raw).unwrap();

    let pages_raw = doc.get_referenced_bytes(root.pages).unwrap();
    // let (_, IntoString(pages)) = extract(pages_raw).unwrap();
    // println!("{pages}");
    let (_, pages) = extract::<PageNode>(pages_raw).unwrap();
    // println!("{pages:?}");

    let page_raw = doc.get_referenced_bytes(pages.kids[0]).unwrap();
    // let (_, IntoString(page)) = extract(page_raw).unwrap();
    let (_, page) = extract::<PageLeaf>(page_raw).unwrap();

    let &content_raw = doc.body.get(&page.contents.0[0]).unwrap();
    // let (_, IntoString(content)) = extract(content_raw).unwrap();
    let (_, content) = extract::<Stream<'_, NoOp>>(content_raw).unwrap();
    let decoded = content.decode().unwrap();
    let decoded = String::from_utf8_lossy(&decoded);
    // println!("{decoded}");

    for line in decoded
        .split('\n')
        .filter(|t| t.to_lowercase().contains("tj"))
    {
        println!("{line:?}");
    }

    // println!("{decoded}");
}
