mod state;
use enum_dispatch::enum_dispatch;
pub use state::{CharSpace, FontSize, HorizontalScale, Leading, RenderMode, Rise, WordSpace};

mod position;
pub use position::{MoveNextLine, MoveTD, MoveTd, TextMatrix};

mod showing;
pub use showing::{ShowApostrophe, ShowQuote, ShowTJ, ShowTj};

use crate::TextState;

#[enum_dispatch]
pub trait Operator {
    fn apply(self, obj: &mut TextState);
}

trait ElementsExtract {
    /// It's more efficient to use a slice of slices:
    /// we can clear the underlying `Vec` and re-use it.
    fn extract_from_elements(elements: &[&[u8]]) -> Self;
}

#[enum_dispatch(Operator)]
enum Op {
    CharSpace,
    WordSpace,
    HorizontalScale,
    FontSize,
    Leading,
    Rise,
    // RenderMode,
    // MoveNextLine,
    // MoveTD,
    // MoveTd,
    // TextMatrix,
    // ShowApostrophe,
    // ShowQuote,
    // ShowTJ,
    // ShowTj,
}

mod parsing;
