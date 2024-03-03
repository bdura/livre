use livre_extraction::{Extract, FromDict, FromDictRef, Parse, RawDict};

#[derive(Debug, Extract, FromDictRef)]
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

    let raw_dict: RawDict<'_> = input.parse().unwrap();
    let test = Test::from_dict(raw_dict).unwrap();

    println!("{:?}", test);
    println!("{:?}", test.n);
    println!("{:?}", test.prev);
}
