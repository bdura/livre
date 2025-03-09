use std::{fs::File, io::Read};

use livre::{
    content::{parse_text_object, Operator},
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

            let mut it = iterator(
                content.as_slice().as_ref(),
                preceded(multispace0, Operator::extract),
            );

            while let Some(mut text_state) = parse_text_object(&mut it).unwrap() {
                println!("NEW TEXT OBJECT");
                for (position, text) in &mut text_state {
                    println!("- {:?} {:?}", position, text);
                }
                println!("END TEXT OBJECT");
                println!();
            }

            let input = &mut it.finish().unwrap().0;

            multispace0::<&BStr, ContextError>(input).unwrap();

            if input.len() != 0 {
                println!();
                println!("Issue while parsing");
                println!(
                    "{}",
                    String::from_utf8_lossy(&input[..500.min(input.len())])
                );
                panic!("Parsing did not consume the entire input");
            }
        }
    }
}
