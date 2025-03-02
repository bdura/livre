use std::{fs::File, io::Read};

use livre::{content::Operator, extraction::Extract, InMemoryDocument};
use rstest::rstest;
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

#[rstest]
#[case("tests/resources/letter.pdf")]
#[case("tests/resources/text.pdf")]
fn test_content(#[case] path: &str) {
    let doc = read_document(path);

    for page in doc.pages().unwrap().iter() {
        let content = page.build_content(&doc).unwrap();

        let mut it = iterator(
            content.as_slice().as_ref(),
            preceded(multispace0, Operator::extract),
        );

        for operator in &mut it {
            println!("{:?}", operator);
        }

        it.finish().unwrap();
    }
}
