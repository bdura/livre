//! Coordinate spaces, as defined in section 8.3 of the PDF specification, carried as type
//! parameters so that a value expressed in one space cannot be used in another.
//!
//! The markers carry no data. A [`Matrix<From, To>`](super::Matrix) maps points of `From`
//! into `To`; composing two matrices whose inner spaces differ, or applying a matrix to a
//! point of another space, does not compile.

use std::{fmt, marker::PhantomData, ops::Add};

/// Text space: the space of the text matrix's source, in which `Td`, `TJ` offsets and glyph
/// advances are expressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Text;

/// User space: the space established by the current transformation matrix, in which `cm`
/// operands and the text matrix's image are expressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct User;

/// Default user space: the page's own space, that of the `MediaBox`. Run positions are
/// reported in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefaultUser;

/// A location in the coordinate space `S`, such as a run position in [`DefaultUser`] space.
///
/// Its coordinates are in the units of `S`. Only a [`Matrix`](super::Matrix) whose source
/// space is `S` maps it into another space.
pub struct Point<S> {
    /// Horizontal coordinate, in units of `S`.
    pub x: f32,
    /// Vertical coordinate, in units of `S`.
    pub y: f32,
    // `fn` so the value owns no `S`: its auto traits and drop check stay independent of
    // the marker.
    space: PhantomData<fn() -> S>,
}

impl<S> Point<S> {
    /// The point `(x, y)` of `S`.
    pub const fn new(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            space: PhantomData,
        }
    }
}

/// A vector in the coordinate space `S`: the difference between two points, such as a `Td`
/// operand or a `TJ` offset in [`Text`] space.
///
/// Adding it to a [`Point`] of the same space moves that point by `(dx, dy)`.
pub struct Displacement<S> {
    /// Horizontal component, in units of `S`.
    pub dx: f32,
    /// Vertical component, in units of `S`.
    pub dy: f32,
    // `fn` so the value owns no `S`: its auto traits and drop check stay independent of
    // the marker.
    space: PhantomData<fn() -> S>,
}

impl<S> Displacement<S> {
    /// The displacement `(dx, dy)` in `S`.
    pub const fn new(dx: f32, dy: f32) -> Self {
        Self {
            dx,
            dy,
            space: PhantomData,
        }
    }
}

impl<S> Add<Displacement<S>> for Point<S> {
    type Output = Point<S>;

    fn add(self, d: Displacement<S>) -> Point<S> {
        Point::new(self.x + d.dx, self.y + d.dy)
    }
}

// Hand-written rather than derived: a derive would bound each impl on `S` (`S: Clone`,
// `S: PartialEq`, ...) although `S` only appears in `PhantomData`. These impls hold for
// every `S`, so code generic over the space needs no such bounds.

impl<S> Clone for Point<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for Point<S> {}

impl<S> PartialEq for Point<S> {
    fn eq(&self, other: &Self) -> bool {
        (self.x, self.y) == (other.x, other.y)
    }
}

impl<S> fmt::Debug for Point<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Point")
            .field("x", &self.x)
            .field("y", &self.y)
            .finish()
    }
}

impl<S> Clone for Displacement<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for Displacement<S> {}

impl<S> PartialEq for Displacement<S> {
    fn eq(&self, other: &Self) -> bool {
        (self.dx, self.dy) == (other.dx, other.dy)
    }
}

impl<S> fmt::Debug for Displacement<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Displacement")
            .field("dx", &self.dx)
            .field("dy", &self.dy)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_plus_displacement() {
        let p: Point<Text> = Point::new(1.0, 2.0) + Displacement::new(10.0, 20.0);
        assert_eq!(p, Point::new(11.0, 22.0));
    }
}
