use livre::{extraction::Extract, InMemoryDocument};
use winnow::BStr;

fn main() {
    //let mut letter: &BStr = include_bytes!("../tests/letter.pdf").as_slice().as_ref();
    let mut letter: &BStr = include_bytes!("../tests/text.pdf").as_slice().as_ref();
    //let mut letter: &BStr = include_bytes!("../resource/ISO_32000-2-2020_sponsored.pdf").as_slice().as_ref();

    let doc = InMemoryDocument::extract(&mut letter).unwrap();

    for page in doc.pages().unwrap().iter() {
        println!(
            "{}",
            String::from_utf8_lossy(&page.build_content(&doc).unwrap())
        )
    }
}
