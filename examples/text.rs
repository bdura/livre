use std::{fs::File, io::Read};

use livre::{content::Operator, extraction::Extract, InMemoryDocument};
use winnow::{
    ascii::multispace0,
    combinator::{iterator, preceded},
};

fn read_document(path: &str) -> InMemoryDocument {
    let mut file = File::open(path).unwrap();
    let mut buffer: Vec<u8> = Vec::new();
    file.read_to_end(&mut buffer).unwrap();

    InMemoryDocument::extract(&mut buffer.as_slice().as_ref()).unwrap()
}

fn main() {
    let paths = &["tests/resources/letter.pdf"];

    for path in paths {
        let doc = read_document(path);

        for page in doc.pages().unwrap().iter() {
            let content = page.build_content(&doc).unwrap();

            let mut it = iterator(
                content.as_slice().as_ref(),
                preceded(multispace0, Operator::extract),
            );

            for operator in &mut it {
                match operator {
                    Operator::BeginText(_) => println!(),
                    Operator::ShowText(v) => print!("{v}"),
                    Operator::ShowTextArray(v) => print!("{v}"),
                    Operator::MoveToNextLineAndShowText(v) => println!("{v}"),
                    Operator::MoveToNextLineAndShowTextWithSpacing(v) => println!("{v}"),
                    _ => {}
                }
            }

            it.finish().unwrap();
        }
    }
}
