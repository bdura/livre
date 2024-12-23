use livre::{
    extraction::{Builder, Extract},
    InMemoryDocument,
};
use winnow::BStr;

fn main() {
    let mut letter: &BStr = include_bytes!("../tests/letter.pdf").as_slice().as_ref();
    //let mut letter: &BStr = include_bytes!("../resource/ISO_32000-2-2020_sponsored.pdf").as_slice().as_ref();
    //let mut letter: &BStr = include_bytes!("../tests/text.pdf").as_slice().as_ref();

    let doc = InMemoryDocument::extract(&mut letter).unwrap();

    for &ref_id in doc.xrefs.keys() {
        println!(
            "{:?}: {:?}",
            ref_id,
            &doc.follow_reference(ref_id).unwrap()[..50]
        )
    }

    //let catalog = doc.build_reference(doc.catalog).unwrap();
    //
    //for page in &catalog.pages.pages {
    //    println!("{page:?}");
    //}
}
