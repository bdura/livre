//! Transformation matrices, as defined in section 8.3.3 of the PDF specification.

use winnow::{combinator::trace, BStr, ModalResult, Parser};

use crate::extraction::{extract, Extract};

/// A transformation matrix, written `[a b c d e f]` in content streams and standing for
///
/// ```text
/// | a b 0 |
/// | c d 0 |
/// | e f 1 |
/// ```
///
/// Coordinates are row vectors, so a point `(x, y)` maps to
/// `(a·x + c·y + e, b·x + d·y + f)`, and the product `M1 × M2` applies `M1` first.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub f: f32,
}

impl Extract<'_> for Matrix {
    fn extract(input: &mut &BStr) -> ModalResult<Self> {
        trace(
            "livre-matrix",
            extract.map(|(a, b, c, d, e, f)| Self { a, b, c, d, e, f }),
        )
        .parse_next(input)
    }
}

impl Default for Matrix {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Matrix {
    /// The identity matrix, `[1 0 0 1 0 0]`.
    pub const IDENTITY: Self = Self {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    /// The product `self × other`: a transformation applying `self` first, then `other`.
    pub fn then(self, other: Self) -> Self {
        Self {
            a: self.a * other.a + self.b * other.c,
            b: self.a * other.b + self.b * other.d,
            c: self.c * other.a + self.d * other.c,
            d: self.c * other.b + self.d * other.d,
            e: self.e * other.a + self.f * other.c + other.e,
            f: self.e * other.b + self.f * other.d + other.f,
        }
    }

    /// Transform a point.
    pub fn apply(&self, (x, y): (f32, f32)) -> (f32, f32) {
        (
            self.a * x + self.c * y + self.e,
            self.b * x + self.d * y + self.f,
        )
    }

    /// The translation components `(e, f)`: where the origin lands.
    pub fn translation(&self) -> (f32, f32) {
        (self.e, self.f)
    }

    /// Translate by `(tx, ty)` in the matrix's own space, i.e. pre-multiply by
    /// `[1 0 0 1 tx ty]`.
    pub fn translate(&mut self, tx: f32, ty: f32) {
        let Self { a, b, c, d, e, f } = *self;

        let e = a * tx + c * ty + e;
        let f = b * tx + d * ty + f;

        *self = Self { a, b, c, d, e, f };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scale(sx: f32, sy: f32) -> Matrix {
        Matrix {
            a: sx,
            b: 0.0,
            c: 0.0,
            d: sy,
            e: 0.0,
            f: 0.0,
        }
    }

    fn translation(tx: f32, ty: f32) -> Matrix {
        Matrix {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: tx,
            f: ty,
        }
    }

    #[test]
    fn product_applies_the_left_operand_first() {
        let point = (1.0, 1.0);

        // Scale, then move: (1, 1) → (2, 3) → (12, 23).
        assert_eq!(
            scale(2.0, 3.0).then(translation(10.0, 20.0)).apply(point),
            (12.0, 23.0)
        );
        // Move, then scale: (1, 1) → (11, 21) → (22, 63).
        assert_eq!(
            translation(10.0, 20.0).then(scale(2.0, 3.0)).apply(point),
            (22.0, 63.0)
        );
    }

    #[test]
    fn identity_is_neutral() {
        let m = Matrix {
            a: 1.0,
            b: 2.0,
            c: 3.0,
            d: 4.0,
            e: 5.0,
            f: 6.0,
        };

        assert_eq!(m.then(Matrix::IDENTITY), m);
        assert_eq!(Matrix::IDENTITY.then(m), m);
        assert_eq!(Matrix::IDENTITY.apply((7.0, 8.0)), (7.0, 8.0));
    }

    #[test]
    fn flip_about_the_page_height() {
        // The `1 0 0 -1 0 792 cm` that Skia emits: y is measured from the top.
        let flip = Matrix {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: -1.0,
            e: 0.0,
            f: 792.0,
        };

        assert_eq!(flip.apply((72.0, 100.0)), (72.0, 692.0));
        assert_eq!(flip.then(flip), Matrix::IDENTITY);
    }

    #[test]
    fn translate_moves_in_the_matrix_own_space() {
        let mut m = scale(2.0, 2.0);
        m.translate(5.0, 7.0);

        assert_eq!(m.translation(), (10.0, 14.0));
        assert_eq!(m, translation(5.0, 7.0).then(scale(2.0, 2.0)));
    }
}
