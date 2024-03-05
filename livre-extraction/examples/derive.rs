use livre_extraction::{parse, Extract, FromDict, FromDictRef, RawDict};

#[derive(Debug, Extract, FromDictRef)]
struct Test {
    n: usize,
    prev: Option<String>,
}

fn main() {
    let input = b"<</N 12/Prev (test)>>";

    let test: Test = parse(input).unwrap();

    println!("{:?}", test);
    println!("{:?}", test.n);
    println!("{:?}", test.prev);

    let raw_dict: RawDict<'_> = parse(input).unwrap();
    let test = Test::from_dict(raw_dict).unwrap();

    println!("{:?}", test);
    println!("{:?}", test.n);
    println!("{:?}", test.prev);
}
