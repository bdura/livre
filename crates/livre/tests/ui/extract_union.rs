use livre::extraction::Extract;

#[derive(Extract)]
union Either {
    a: i32,
    b: f32,
}

fn main() {}
