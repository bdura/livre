use livre::extraction::FromRawDict;

#[derive(FromRawDict)]
enum Choice {
    A(i32),
    B(bool),
}

fn main() {}
