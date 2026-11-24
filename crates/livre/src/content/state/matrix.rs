//! Transformation matrices, as defined in section 8.3.3 of the PDF specification.

use std::{fmt, marker::PhantomData};

use winnow::{combinator::trace, BStr, ModalResult, Parser};

use super::space::{Displacement, Glyph, Point, Text};
use crate::{
    content::font::GlyphWidth,
    extraction::{extract, Extract},
};

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
/// The spaces are markers such as [`Text`] and [`User`](super::User):
/// [`then`](Self::then) only composes matrices whose inner spaces agree, and
/// [`apply`](Self::apply) only takes points of `From`.
pub struct Matrix<From, To> {
    /// Horizontal scaling component: the weight of `x` in the image's `x`.
    pub a: f32,
    /// Rotation/skew component: the weight of `x` in the image's `y`.
    pub b: f32,
    /// Rotation/skew component: the weight of `y` in the image's `x`.
    pub c: f32,
    /// Vertical scaling component: the weight of `y` in the image's `y`.
    pub d: f32,
    /// Horizontal translation, in units of `To`.
    pub e: f32,
    /// Vertical translation, in units of `To`.
    pub f: f32,
    // `fn` so a matrix owns neither space: its auto traits and drop check stay
    // independent of the markers.
    spaces: PhantomData<fn(From) -> To>,
}

/// Six numbers `a b c d e f`, as [`Matrix::new`] takes them.
///
/// The operands say nothing about spaces: the declared type asserts them, as with
/// [`Matrix::new`]. Extract into a field whose type names both spaces, never into a binding
/// left to inference, which would pick whatever spaces make the surrounding code compile.
impl<From, To> Extract<'_> for Matrix<From, To> {
    fn extract(input: &mut &BStr) -> ModalResult<Self> {
        trace(
            "livre-matrix",
            extract.map(|(a, b, c, d, e, f)| Self::new(a, b, c, d, e, f)),
        )
        .parse_next(input)
    }
}

/// The six numbers of a PDF array `[a b c d e f]`, such as a font's `/FontMatrix`, as
/// [`Matrix::new`] takes them.
///
/// As with [`Matrix::new`], the declared type asserts the spaces: convert into a field or
/// binding whose type names both.
impl<F, T> std::convert::From<[f32; 6]> for Matrix<F, T> {
    fn from([a, b, c, d, e, f]: [f32; 6]) -> Self {
        Self::new(a, b, c, d, e, f)
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

impl<From, To> Matrix<From, To> {
    /// The matrix `[a b c d e f]`, in the row-vector convention of section 8.3.4 of the
    /// PDF specification: `(x, y)` maps to `(a·x + c·y + e, b·x + d·y + f)`.
    ///
    /// The spaces are not checked: the caller asserts that the six numbers map `From` into
    /// `To`. This is the one place a space pair is minted: every other constructor goes
    /// through it, and [`then`](Self::then) only combines pairs minted here.
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
    pub fn then<Next>(self, other: Matrix<To, Next>) -> Matrix<From, Next> {
        Matrix::new(
            self.a * other.a + self.b * other.c,
            self.a * other.b + self.b * other.d,
            self.c * other.a + self.d * other.c,
            self.c * other.b + self.d * other.d,
            self.e * other.a + self.f * other.c + other.e,
            self.e * other.b + self.f * other.d + other.f,
        )
    }

    /// Map a point of `From` to the corresponding point of `To`.
    pub fn apply(&self, point: Point<From>) -> Point<To> {
        let Point { x, y, .. } = point;
        Point::new(
            self.a * x + self.c * y + self.e,
            self.b * x + self.d * y + self.f,
        )
    }

    /// The translation components `(e, f)`: where the origin of `From` lands.
    pub fn translation(&self) -> Point<To> {
        Point::new(self.e, self.f)
    }

    /// Translate by `displacement` in the matrix's source space, i.e. pre-multiply by
    /// `[1 0 0 1 dx dy]`.
    pub fn translate(&mut self, displacement: Displacement<From>) {
        let Displacement { dx, dy, .. } = displacement;

        self.e += self.a * dx + self.c * dy;
        self.f += self.b * dx + self.d * dy;
    }
}

impl Matrix<Glyph, Text> {
    /// The text space displacement of a glyph `width` along the glyph's horizontal axis:
    /// the linear part of the matrix applied to `(width, 0)`, without its translation.
    pub fn apply_width(&self, width: GlyphWidth) -> Displacement<Text> {
        let GlyphWidth(w) = width;
        Displacement::new(self.a * w, self.b * w)
    }
}

// Hand-written rather than derived: a derive would bound each impl on the spaces
// (`From: Clone`, `To: PartialEq`, ...) although they only appear in `PhantomData`. These
// impls hold for every pair of spaces, so code generic over them needs no such bounds.

impl<From, To> Clone for Matrix<From, To> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<From, To> Copy for Matrix<From, To> {}

impl<From, To> PartialEq for Matrix<From, To> {
    fn eq(&self, o: &Self) -> bool {
        (self.a, self.b, self.c, self.d, self.e, self.f) == (o.a, o.b, o.c, o.d, o.e, o.f)
    }
}

impl<From, To> fmt::Debug for Matrix<From, To> {
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
    use crate::content::state::User;

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
}
