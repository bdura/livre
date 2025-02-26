/// `BT` operator.
///
/// Begin a text object, initializing the text matrix $T_m$ and the text line matrix
/// $T_{lm}$, to the identity matrix. Text objects shall not be nested; a second
/// `BeginText` shall not apear before an [`EndText`](EndText).
pub struct BeginText;

/// `ET` operator.
///
/// End a text object, discarding the text matrix.
pub struct EndText;
