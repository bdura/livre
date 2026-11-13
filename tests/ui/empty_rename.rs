use livre::extraction::FromRawDict;

#[derive(FromRawDict)]
struct Case {
    #[livre(rename = "")]
    field: i32,
}

fn main() {}
