//! The CTM takes user-space points, not text-space ones.
use livre::content::{DefaultUser, Matrix, Point, Text, User};

fn main() {
    let ctm: Matrix<User, DefaultUser> = Matrix::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
    let point: Point<Text> = Point::new(1.0, 2.0);

    let _ = ctm.apply(point);
}
