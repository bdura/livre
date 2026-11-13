use livre::extraction::FromRawDict;

#[derive(FromRawDict)]
struct Case {
    #[livre(rename = "A", rename = "B")]
    field: i32,
}

fn main() {}
