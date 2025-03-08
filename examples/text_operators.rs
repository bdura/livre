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
    BStr, Parser,
};

fn read_document(path: &str) -> InMemoryDocument {
    let mut file = File::open(path).unwrap();
    let mut buffer: Vec<u8> = Vec::new();
    file.read_to_end(&mut buffer).unwrap();

    InMemoryDocument::extract(&mut buffer.as_slice().as_ref()).unwrap()
}

fn main() {
    let paths = &[
        "tests/resources/letter.pdf",
        "tests/resources/text.pdf",
        "/Users/basile/Documents/Books/pdf/ISO_32000-2_sponsored-ec2.pdf",
    ];

    for path in paths {
        let doc = read_document(path);

        for page in doc.pages().unwrap().iter() {
            let content = page.build_content(&doc).unwrap();

            let mut it = iterator(
                content.as_slice().as_ref(),
                preceded(multispace0, Operator::extract),
            );

            while let Some(mut text_state) = parse_text_object(&mut it) {
                println!("NEW TEXT OBJECT");
                for (position, c) in &mut text_state {
                    println!("- {:?} {:?}", position, char::from(c));
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
