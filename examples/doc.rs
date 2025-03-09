use std::{fs::File, io::Write};

use livre::{extraction::Extract, InMemoryDocument};
use winnow::BStr;

fn main() {
    // let mut letter: &BStr = include_bytes!("../tests/letter.pdf").as_slice().as_ref();
    let mut letter: &BStr = include_bytes!("../tests/resources/text.pdf")
        .as_slice()
        .as_ref();
    //let mut letter: &BStr = include_bytes!("../resource/ISO_32000-2-2020_sponsored.pdf").as_slice().as_ref();

    let doc = InMemoryDocument::extract(&mut letter).unwrap();
    let mut output = File::create("./tests/test.txt").unwrap();

    for page in doc.pages().unwrap().iter() {
        output
            .write_all(&page.build_content(&doc).unwrap())
            .unwrap();
        // write!(&output, &page.build_content(&doc).unwrap());
        // write!(
        //     "{}",
        //     String::from_utf8_lossy(&page.build_content(&doc).unwrap())
        // )
    }
}
