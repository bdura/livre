use std::{fs::File, io::Read};

use livre::{
    content::{operators::Operator, parse_text_object},
    extraction::Extract,
    InMemoryDocument,
};
use winnow::{
    ascii::multispace0,
    combinator::{iterator, preceded},
    error::ContextError,
    BStr,
};

fn read_document(path: &str) -> InMemoryDocument {
    let mut file = File::open(path).unwrap();
    let mut buffer: Vec<u8> = Vec::new();
    file.read_to_end(&mut buffer).unwrap();

    InMemoryDocument::extract(&mut buffer.as_slice().as_ref()).unwrap()
}

fn main() {
    let paths = &["tests/resources/text.pdf", "tests/resources/letter.pdf"];

    for path in paths {
        let doc = read_document(path);

        for page in doc.pages().unwrap().iter() {
            let content = page.build_content(&doc).unwrap();
            let mut stream = BStr::new(&content);

            let mut it = iterator(&mut stream, preceded(multispace0, Operator::extract));

            while let Some(mut text_state) = parse_text_object(&mut it).unwrap() {
                println!("NEW TEXT OBJECT");
                for (position, text) in &mut text_state {
                    println!("- {:?} {:?}", position, text);
                }
                println!("END TEXT OBJECT");
                println!();
            }

            multispace0::<&BStr, ContextError>(&mut stream).unwrap();

            if !stream.is_empty() {
                println!();
                println!("Issue while parsing");
                println!(
                    "{}",
                    String::from_utf8_lossy(&stream[..500.min(stream.len())])
                );
                panic!("Parsing did not consume the entire input");
            }
        }
    }
}
