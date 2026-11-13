use livre::extraction::FromRawDict;

#[derive(FromRawDict)]
struct Unknown {
    #[livre(bogus)]
    field: i32,
}

fn main() {}
