use std::{
    fs::File,
    io::{prelude::*, BufReader},
};

use pdf_parser::{
    error::ParsingError,
    objects::{Dictionary, Reference},
    structure::Document,
};

fn main() {
    let file = File::open("examples/text.pdf").unwrap();
    let mut reader = BufReader::new(file);

    let mut buf: Vec<u8> = Vec::new();
    reader.read_to_end(&mut buf).ok();

    // reader.seek(std::io::SeekFrom::End(-100)).unwrap();

    let (_, doc) = Document::parse(&buf)
        .map_err(|_| ParsingError::ParsingError("test".to_string()))
        .unwrap();
    let dict: Dictionary = doc
        .body
        .get(&Reference::new(2, 0))
        .unwrap()
        .clone()
        .try_into()
        .unwrap();
    println!("{dict:#?}");
}
