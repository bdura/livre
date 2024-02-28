use livre_extraction::{extraction::Parse, Extract};

#[derive(Debug, Extract)]
struct Test {
    #[livre(rename = "N")]
    n: usize,
    #[livre(rename = "Prev")]
    prev: String,
}

fn main() {
    let input = b"<</N 12/Prev (test)>>";

    let test: Test = input.parse().unwrap();

    println!("{test:?}")
}
