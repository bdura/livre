use livre::{
    extraction::{Extract, Object},
    follow_refs::Builder,
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
            &doc.build_reference::<Object>(ref_id.into()).unwrap()
        )
    }

    //let catalog = doc.build_reference(doc.catalog).unwrap();
    //
    //for page in &catalog.pages.pages {
    //    println!("{page:?}");
    //}
}
