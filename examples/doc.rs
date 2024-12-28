use livre::{
    extraction::{Builder, Extract, Object, Reference},
    InMemoryDocument,
};
use winnow::BStr;

fn main() {
    let mut letter: &BStr = include_bytes!("../tests/letter.pdf").as_slice().as_ref();
    //let mut letter: &BStr = include_bytes!("../resource/ISO_32000-2-2020_sponsored.pdf").as_slice().as_ref();
    //let mut letter: &BStr = include_bytes!("../tests/text.pdf").as_slice().as_ref();

    let doc = InMemoryDocument::extract(&mut letter).unwrap();

    for &ref_id in doc.xrefs.keys() {
        let reference: Reference<Object> = ref_id.into();
        if let Ok(object) = doc.build_reference(reference) {
            let mut debug = format!("{:?}", object);
            debug.truncate(100);
            println!("{:?}\n{:}\n", reference.id, debug);
        } else {
            println!("Issue while trying to build ref {:?}", reference)
        }
    }

    //let catalog = doc.build_reference(doc.catalog).unwrap();
    //
    //for page in &catalog.pages.pages {
    //    println!("{page:?}");
    //}
}
