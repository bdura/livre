use livre_extraction::{extraction::Parse, Extract};

#[derive(Debug, Extract)]
struct Test {
    n: usize,
    prev: Option<String>,
}

fn main() {
    let input = b"<</N 12/Prev (test)>>";

    let test: Test = input.parse().unwrap();

    println!("{:?}", test);
    println!("{:?}", test.n);
    println!("{:?}", test.prev);
}
