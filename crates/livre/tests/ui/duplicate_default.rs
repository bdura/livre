use livre::extraction::FromRawDict;

#[derive(FromRawDict)]
struct Case {
    #[livre(default, default = 1)]
    field: i32,
}

fn main() {}
