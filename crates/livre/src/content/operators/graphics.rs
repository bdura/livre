//! Special graphics state operators: `q`, `Q` and `cm`. See section 8.4.4 of the PDF
//! specification.
//!
//! These operators are only allowed at the page description level, outside text objects,
//! so the current transformation matrix is constant within a text object.

use enum_dispatch::enum_dispatch;

use crate::{
    content::state::{GraphicsState, Matrix},
    extraction::Extract,
};

use super::Operator;

/// Abstraction over the special graphics state operators, as defined in section 8.4.4 of
/// the PDF specification.
/// Defines operations that act on the [`GraphicsState`].
#[enum_dispatch]
pub trait GraphicsStateOperation: Sized {
    fn apply(self, state: &mut GraphicsState);
}

#[derive(Debug, Clone, PartialEq)]
#[enum_dispatch(GraphicsStateOperation)]
pub enum GraphicsStateOperator {
    /// The `q` operator. Save the current graphics state on the graphics state stack.
    SaveGraphicsState(SaveGraphicsState),
    /// The `Q` operator. Restore the graphics state from the graphics state stack.
    RestoreGraphicsState(RestoreGraphicsState),
    /// The `cm` operator. Concatenate a matrix to the current transformation matrix.
    ConcatenateMatrix(ConcatenateMatrix),
}

/// `q` operator.
///
/// > Save the current graphics state on the graphics state stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Extract)]
pub struct SaveGraphicsState;

impl GraphicsStateOperation for SaveGraphicsState {
    fn apply(self, state: &mut GraphicsState) {
        state.save();
    }
}

/// `Q` operator.
///
/// > Restore the graphics state by removing the most recently saved state from the stack
/// > and making it the current state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Extract)]
pub struct RestoreGraphicsState;

impl GraphicsStateOperation for RestoreGraphicsState {
    fn apply(self, state: &mut GraphicsState) {
        state.restore();
    }
}

/// `cm` operator.
///
/// > Modify the current transformation matrix (CTM) by concatenating the specified matrix.
/// > Although the operands specify a matrix, they shall be written as six separate numbers,
/// > not as an array.
///
/// ```raw
/// 1 0 0 -1 0 792 cm
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Extract)]
pub struct ConcatenateMatrix(pub Matrix);

impl GraphicsStateOperation for ConcatenateMatrix {
    fn apply(self, state: &mut GraphicsState) {
        state.concatenate(self.0);
    }
}

impl From<GraphicsStateOperator> for Operator {
    fn from(op: GraphicsStateOperator) -> Self {
        Operator::GraphicsState(op)
    }
}

macro_rules! impl_from {
    ($($t:ident,)+) => {
        $(
            impl From<$t> for Operator {
                fn from(op: $t) -> Self {
                    Operator::GraphicsState(GraphicsStateOperator::$t(op))
                }
            }
        )+
    };
}

impl_from!(SaveGraphicsState, RestoreGraphicsState, ConcatenateMatrix,);
