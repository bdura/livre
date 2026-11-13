//! The graphics state tracked between text objects. See section 8.4 of the PDF
//! specification.

use super::Matrix;

/// The part of the graphics state that livre tracks: the current transformation matrix
/// (CTM) and the stack that `q` and `Q` push to and pop from.
///
/// The CTM starts as the identity, so positions derived from it are expressed in the
/// page's default user space (the space of the `MediaBox`). The other parameters of the
/// graphics state (colour, line width, clipping, text state set outside `BT`) are not
/// tracked yet.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GraphicsState {
    ctm: Matrix,
    stack: Vec<Matrix>,
}

impl GraphicsState {
    /// The current transformation matrix: user space to default user space.
    pub fn ctm(&self) -> Matrix {
        self.ctm
    }

    /// The `cm` operator: `CTM' = M × CTM`, so `matrix` applies before the current CTM.
    pub fn concatenate(&mut self, matrix: Matrix) {
        self.ctm = matrix.then(self.ctm);
    }

    /// The `q` operator.
    pub fn save(&mut self) {
        self.stack.push(self.ctm);
    }

    /// The `Q` operator. A `Q` without a matching `q` is an error in the specification;
    /// livre ignores it and keeps the current state.
    pub fn restore(&mut self) {
        match self.stack.pop() {
            Some(ctm) => self.ctm = ctm,
            None => tracing::warn!("`Q` without a matching `q`: ignored"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn translation(tx: f32, ty: f32) -> Matrix {
        Matrix {
            e: tx,
            f: ty,
            ..Matrix::IDENTITY
        }
    }

    #[test]
    fn restore_pops_the_state_saved_by_the_matching_save() {
        let mut state = GraphicsState::default();

        state.concatenate(translation(1.0, 0.0));
        state.save();
        state.concatenate(translation(10.0, 0.0));
        state.save();
        state.concatenate(translation(100.0, 0.0));
        assert_eq!(state.ctm().translation(), (111.0, 0.0));

        state.restore();
        assert_eq!(state.ctm().translation(), (11.0, 0.0));
        state.restore();
        assert_eq!(state.ctm().translation(), (1.0, 0.0));
    }

    #[test]
    fn unbalanced_restore_keeps_the_state() {
        let mut state = GraphicsState::default();
        state.concatenate(translation(1.0, 2.0));

        state.restore();

        assert_eq!(state.ctm().translation(), (1.0, 2.0));
    }
}
