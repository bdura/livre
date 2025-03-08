use winnow::{BStr, PResult};

use crate::{content::operators::behavior::TextOperation, extraction::Extract};

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

impl Extract<'_> for BeginText {
    fn extract(_input: &mut &BStr) -> PResult<Self> {
        Ok(Self)
    }
}

impl Extract<'_> for EndText {
    fn extract(_input: &mut &BStr) -> PResult<Self> {
        Ok(Self)
    }
}

impl TextOperation for BeginText {}
impl TextOperation for EndText {}
