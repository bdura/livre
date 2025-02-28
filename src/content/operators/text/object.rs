use crate::content::operators::FromArgs;

/// `BT` operator.
///
/// Begin a text object, initializing the text matrix $T_m$ and the text line matrix
/// $T_{lm}$, to the identity matrix. Text objects shall not be nested; a second
/// `BeginText` shall not apear before an [`EndText`](EndText).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BeginText;

/// `ET` operator.
///
/// End a text object, discarding the text matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndText;

impl FromArgs for BeginText {
    fn from_args(
        _: &mut Vec<crate::extraction::Object>,
    ) -> Result<Self, crate::content::operators::OperatorError> {
        Ok(BeginText)
    }
}

impl FromArgs for EndText {
    fn from_args(
        _: &mut Vec<crate::extraction::Object>,
    ) -> Result<Self, crate::content::operators::OperatorError> {
        Ok(EndText)
    }
}
