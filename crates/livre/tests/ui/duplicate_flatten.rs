use livre::extraction::FromRawDict;

#[derive(FromRawDict)]
struct Case {
    #[livre(flatten)]
    #[livre(flatten)]
    inner: Inner,
}

#[derive(FromRawDict)]
struct Inner {
    field: i32,
}

fn main() {}
