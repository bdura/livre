//! `text-xref-stream.pdf` is `text.pdf` rewritten by qpdf with object streams and a
//! cross-reference stream encoded with `/Predictor 12`: both must read the same.

use std::{fs::File, io::Read};

use livre::{extraction::Extract, InMemoryDocument};

fn read_document(path: &str) -> InMemoryDocument {
    let mut file = File::open(path).unwrap();
    let mut buffer: Vec<u8> = Vec::new();
    file.read_to_end(&mut buffer).unwrap();

    InMemoryDocument::extract(&mut buffer.as_slice().as_ref()).unwrap()
}

#[test]
fn predicted_xref_stream_reads_like_the_original() {
    let original = read_document("tests/resources/text.pdf");
    let rewritten = read_document("tests/resources/text-xref-stream.pdf");

    let original_pages = original.pages().unwrap();
    let rewritten_pages = rewritten.pages().unwrap();

    assert_eq!(original_pages.len(), rewritten_pages.len());

    for (i, (expected, actual)) in original_pages
        .iter()
        .zip(rewritten_pages.iter())
        .enumerate()
    {
        assert_eq!(
            expected.build_content(&original).unwrap(),
            actual.build_content(&rewritten).unwrap(),
            "page {i} differs",
        );
    }
}
