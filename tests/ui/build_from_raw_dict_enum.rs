use livre::follow_refs::BuildFromRawDict;

#[derive(BuildFromRawDict)]
enum Choice {
    A(i32),
    B(bool),
}

fn main() {}
