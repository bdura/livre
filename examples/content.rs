use livre::{content::Operator, extraction::Extract, InMemoryDocument};
use winnow::{
    ascii::multispace0,
    combinator::{iterator, preceded},
    BStr, Parser,
};

fn main() {
    let mut letter: &BStr = include_bytes!("../tests/resources/letter.pdf")
        .as_slice()
        .as_ref();
    // let mut letter: &BStr = include_bytes!("../tests/text.pdf").as_slice().as_ref();
    //let mut letter: &BStr = include_bytes!("../resource/ISO_32000-2-2020_sponsored.pdf").as_slice().as_ref();

    let doc = InMemoryDocument::extract(&mut letter).unwrap();

    for page in doc.pages().unwrap().iter() {
        let content = page.build_content(&doc).unwrap();

        let mut it = iterator(
            content.as_slice().as_ref(),
            preceded(multispace0, Operator::extract.with_taken()),
        );

        for (operator, slice) in &mut it {
            println!("{:>20} {:?}", String::from_utf8_lossy(slice), operator);
        }

        it.finish().unwrap();
    }
}
