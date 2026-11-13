use livre::extraction::FromRawDict;

#[derive(FromRawDict)]
struct Case {
    #[livre(default)]
    field: Option<i32>,
}

fn main() {}
