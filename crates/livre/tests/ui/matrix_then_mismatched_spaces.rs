//! Composing in the wrong order: the CTM maps to default user space, but the text matrix
//! starts in text space.
use livre::content::{DefaultUser, Matrix, Text, User};

fn main() {
    let tm: Matrix<Text, User> = Matrix::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
    let ctm: Matrix<User, DefaultUser> = Matrix::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);

    let _ = ctm.then(tm);
}
