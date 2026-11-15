use std::{fs::File, io::Read};

use livre::{
    content::operators::Operator,
    extraction::{multicomment0, Extract},
    InMemoryDocument,
};
use rstest::rstest;
use winnow::{
    combinator::{eof, iterator, preceded},
    BStr, Parser,
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
#[case("tests/resources/text-xref-stream.pdf")]
fn test_content(#[case] path: &str) {
    let doc = read_document(path);

    for page in doc.pages().unwrap().iter() {
        let content = page.build_content(&doc).unwrap();

        let mut stream = BStr::new(&content);

        let mut it = iterator(&mut stream, preceded(multicomment0, Operator::extract));

        for operator in &mut it {
            println!("{:?}", operator);
        }

        it.finish().unwrap();

        (multicomment0, eof).parse_next(&mut stream).unwrap();
    }
}
