use livre::extraction::FromRawDict;

#[derive(FromRawDict)]
struct Case {
    #[livre(flatten, rename = "Inner")]
    inner: Inner,
}

#[derive(FromRawDict)]
struct Inner {
    field: i32,
}

fn main() {}
