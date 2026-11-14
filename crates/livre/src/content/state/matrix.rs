//! Transformation matrices, as defined in section 8.3.3 of the PDF specification.

use std::{fmt, marker::PhantomData};

use winnow::{combinator::trace, BStr, ModalResult, Parser};

use super::space::{Displacement, Point, User};
use crate::extraction::{extract, Extract};

/// A transformation matrix from the space `From` to the space `To`, written `[a b c d e f]`
/// in content streams and standing for
///
/// ```text
/// | a b 0 |
/// | c d 0 |
/// | e f 1 |
/// ```
///
/// Coordinates are row vectors, so a point `(x, y)` maps to
/// `(a·x + c·y + e, b·x + d·y + f)`, and the product `M1 × M2` applies `M1` first.
///
/// The spaces are markers such as [`Text`](super::Text) and [`User`](super::User):
/// [`then`](Self::then) only composes matrices whose inner spaces agree, and
/// [`apply`](Self::apply) only takes points of `From`.
pub struct Matrix<From, To> {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub f: f32,
    spaces: PhantomData<fn(From) -> To>,
}

/// `cm`'s operand: six numbers, concatenated to the CTM in user space. The `Tm` operator
/// assembles its text matrix from six numbers itself, so this is the only extractable
/// matrix.
impl Extract<'_> for Matrix<User, User> {
    fn extract(input: &mut &BStr) -> ModalResult<Self> {
        trace(
            "livre-matrix",
            extract.map(|(a, b, c, d, e, f)| Self::new(a, b, c, d, e, f)),
        )
        .parse_next(input)
    }
}

impl<A> Default for Matrix<A, A> {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl<A> Matrix<A, A> {
    /// The identity matrix, `[1 0 0 1 0 0]`.
    pub const IDENTITY: Self = Self::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
}

impl<A, B> Matrix<A, B> {
    /// The matrix `[a b c d e f]`.
    pub const fn new(a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) -> Self {
        Self {
            a,
            b,
            c,
            d,
            e,
            f,
            spaces: PhantomData,
        }
    }

    /// The identity between two distinct spaces that coincide at this point of the
    /// content stream: user and default user space before any `cm`, text and user space
    /// at `BT`.
    pub(crate) const fn coincident() -> Self {
        Self::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)
    }

    /// The product `self × other`: a transformation applying `self` first, then `other`.
    pub fn then<C>(self, other: Matrix<B, C>) -> Matrix<A, C> {
        Matrix::new(
            self.a * other.a + self.b * other.c,
            self.a * other.b + self.b * other.d,
            self.c * other.a + self.d * other.c,
            self.c * other.b + self.d * other.d,
            self.e * other.a + self.f * other.c + other.e,
            self.e * other.b + self.f * other.d + other.f,
        )
    }

    /// Transform a point.
    pub fn apply(&self, point: Point<A>) -> Point<B> {
        let Point { x, y, .. } = point;
        Point::new(
            self.a * x + self.c * y + self.e,
            self.b * x + self.d * y + self.f,
        )
    }

    /// The translation components `(e, f)`: where the origin of `From` lands.
    pub fn translation(&self) -> Point<B> {
        Point::new(self.e, self.f)
    }

    /// Translate by `displacement` in the matrix's source space, i.e. pre-multiply by
    /// `[1 0 0 1 dx dy]`.
    pub fn translate(&mut self, displacement: Displacement<A>) {
        let Displacement { dx, dy, .. } = displacement;

        self.e += self.a * dx + self.c * dy;
        self.f += self.b * dx + self.d * dy;
    }
}

// Hand-written so that the space markers need not implement the traits themselves.

impl<A, B> Clone for Matrix<A, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, B> Copy for Matrix<A, B> {}

impl<A, B> PartialEq for Matrix<A, B> {
    fn eq(&self, o: &Self) -> bool {
        (self.a, self.b, self.c, self.d, self.e, self.f) == (o.a, o.b, o.c, o.d, o.e, o.f)
    }
}

impl<A, B> fmt::Debug for Matrix<A, B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Matrix")
            .field("a", &self.a)
            .field("b", &self.b)
            .field("c", &self.c)
            .field("d", &self.d)
            .field("e", &self.e)
            .field("f", &self.f)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::state::space::{DefaultUser, Text};

    type UserMatrix = Matrix<User, User>;

    fn scale(sx: f32, sy: f32) -> UserMatrix {
        Matrix::new(sx, 0.0, 0.0, sy, 0.0, 0.0)
    }

    fn translation(tx: f32, ty: f32) -> UserMatrix {
        Matrix::new(1.0, 0.0, 0.0, 1.0, tx, ty)
    }

    #[test]
    fn product_applies_the_left_operand_first() {
        let point = Point::new(1.0, 1.0);

        // Scale, then move: (1, 1) → (2, 3) → (12, 23).
        assert_eq!(
            scale(2.0, 3.0).then(translation(10.0, 20.0)).apply(point),
            Point::new(12.0, 23.0)
        );
        // Move, then scale: (1, 1) → (11, 21) → (22, 63).
        assert_eq!(
            translation(10.0, 20.0).then(scale(2.0, 3.0)).apply(point),
            Point::new(22.0, 63.0)
        );
    }

    #[test]
    fn identity_is_neutral() {
        let m: UserMatrix = Matrix::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0);

        assert_eq!(m.then(Matrix::IDENTITY), m);
        assert_eq!(Matrix::IDENTITY.then(m), m);
        assert_eq!(
            UserMatrix::IDENTITY.apply(Point::new(7.0, 8.0)),
            Point::new(7.0, 8.0)
        );
    }

    #[test]
    fn flip_about_the_page_height() {
        // The `1 0 0 -1 0 792 cm` that Skia emits: y is measured from the top.
        let flip: UserMatrix = Matrix::new(1.0, 0.0, 0.0, -1.0, 0.0, 792.0);

        assert_eq!(flip.apply(Point::new(72.0, 100.0)), Point::new(72.0, 692.0));
        assert_eq!(flip.then(flip), Matrix::IDENTITY);
    }

    #[test]
    fn translate_moves_in_the_matrix_own_space() {
        let mut m = scale(2.0, 2.0);
        m.translate(Displacement::new(5.0, 7.0));

        assert_eq!(m.translation(), Point::new(10.0, 14.0));
        assert_eq!(m, translation(5.0, 7.0).then(scale(2.0, 2.0)));
    }

    /// The product as computed before the matrices were typed, on raw `[a b c d e f]`.
    fn untyped_product(m: [f32; 6], n: [f32; 6]) -> [f32; 6] {
        let [a, b, c, d, e, f] = m;
        let [na, nb, nc, nd, ne, nf] = n;
        [
            a * na + b * nc,
            a * nb + b * nd,
            c * na + d * nc,
            c * nb + d * nd,
            e * na + f * nc + ne,
            e * nb + f * nd + nf,
        ]
    }

    #[test]
    fn typed_product_matches_the_untyped_one() {
        let (sin, cos) = 30f32.to_radians().sin_cos();
        let rotation = [cos, sin, -sin, cos, 3.0, 4.0];
        let shift = [1.0, 0.0, 0.0, 1.0, 72.0, 792.0];

        let [a, b, c, d, e, f] = rotation;
        let tm: Matrix<Text, User> = Matrix::new(a, b, c, d, e, f);
        let [a, b, c, d, e, f] = shift;
        let ctm: Matrix<User, DefaultUser> = Matrix::new(a, b, c, d, e, f);

        let product: Matrix<Text, DefaultUser> = tm.then(ctm);
        let [a, b, c, d, e, f] = untyped_product(rotation, shift);

        assert_eq!(product, Matrix::new(a, b, c, d, e, f));
        // Not the identity, so the check above is not vacuous.
        assert_ne!(product.a, 1.0);
    }
}
