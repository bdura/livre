use crate::extraction::Extract;

/// `BT` operator. Marks the beginning of a text object.
///
/// From the specification:
///
/// > Begin a text object, initializing the text matrix $T_m$ and the text line matrix
/// > $T_{lm}$, to the identity matrix. Text objects shall not be nested; a second
/// > `BeginText` shall not apear before an [`EndText`](EndText).
///
/// In Livre, when the iterator over the operators encounters a `BT` tag, we generate a
/// text object and starts adding text operators to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Extract)]
pub struct BeginText;

/// `ET` operator. Marks the end of a text object.
///
/// > End a text object, discarding the text matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Extract)]
pub struct EndText;
