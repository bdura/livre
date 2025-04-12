use std::{fs::File, io::Read};

use livre::{
    extraction::{Extract, Map, Object, Reference, ReferenceId},
    follow_refs::Builder,
    InMemoryDocument,
};

fn read_document(path: &str) -> InMemoryDocument {
    let mut file = File::open(path).unwrap();
    let mut buffer: Vec<u8> = Vec::new();
    file.read_to_end(&mut buffer).unwrap();

    InMemoryDocument::extract(&mut buffer.as_slice().as_ref()).unwrap()
}

fn iter_dictionaries(
    document: &InMemoryDocument,
) -> impl Iterator<Item = (ReferenceId, Map<Object>)> + '_ {
    document
        .builder
        .xrefs
        .keys()
        .copied()
        .flat_map(|reference_id| {
            let reference = Reference::<Map<Object>>::from(reference_id);
            document
                .build_reference(reference)
                .ok()
                .map(|d| (reference_id, d))
        })
}

fn main() {
    let doc = read_document("tests/resources/text.pdf");

    for (reference_id, dictionary) in iter_dictionaries(&doc).filter(|(_, d)| {
        d.get(&b"Type".into())
            .is_some_and(|obj| obj == &Object::Name(b"Font".into()))
    }) {
        println!();
        println!("{reference_id:?}");
        for (key, value) in &dictionary {
            println!("{key:?}: {value:?}");
        }
    }
}
