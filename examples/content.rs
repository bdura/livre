use std::{fs::File, io::Read};

use livre::{content::operators::Operator, extraction::Extract, InMemoryDocument};
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
    let paths = &["tests/resources/letter.pdf", "tests/resources/text.pdf"];

    for path in paths {
        let doc = read_document(path);

        for page in doc.pages().unwrap().iter() {
            let content = page.build_content(&doc).unwrap();
            let mut stream = BStr::new(&content);

            let mut it = iterator(
                &mut stream,
                preceded(multispace0, Operator::extract.with_taken()),
            );

            for (operator, slice) in &mut it {
                println!("{:>20} {:?}", String::from_utf8_lossy(slice), operator);
            }

            it.finish().unwrap();

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
