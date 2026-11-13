use livre::extraction::Extract;

#[derive(Extract)]
struct Pair(#[livre(rename = "A")] i32, i32);

fn main() {}
