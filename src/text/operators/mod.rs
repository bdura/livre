mod state;
use enum_dispatch::enum_dispatch;
use nom::{branch::alt, combinator::map};
pub use state::{CharSpace, FontSize, HorizontalScale, Leading, RenderMode, Rise, WordSpace};

mod position;
pub use position::{MoveNextLine, MoveTD, MoveTd, TextMatrix};

mod showing;
pub use showing::{ShowApostrophe, ShowQuote, ShowTJ, ShowTj};

use crate::parsers::Extract;

use super::TextState;

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
#[derive(Debug)]
pub enum Op {
    CharSpace,
    WordSpace,
    HorizontalScale,
    FontSize,
    Leading,
    Rise,
    RenderMode,
    MoveNextLine,
    MoveTD,
    MoveTd,
    TextMatrix,
    ShowApostrophe,
    ShowQuote,
    ShowTJ,
    ShowTj,
}

impl Extract<'_> for Op {
    fn extract(input: &[u8]) -> nom::IResult<&[u8], Self> {
        alt((
            map(CharSpace::extract, Self::CharSpace),
            map(WordSpace::extract, Self::WordSpace),
            map(HorizontalScale::extract, Self::HorizontalScale),
            map(FontSize::extract, Self::FontSize),
            map(Leading::extract, Self::Leading),
            map(Rise::extract, Self::Rise),
            map(RenderMode::extract, Self::RenderMode),
            map(MoveNextLine::extract, Self::MoveNextLine),
            map(MoveTD::extract, Self::MoveTD),
            map(MoveTd::extract, Self::MoveTd),
            map(TextMatrix::extract, Self::TextMatrix),
            map(ShowApostrophe::extract, Self::ShowApostrophe),
            map(ShowQuote::extract, Self::ShowQuote),
            map(ShowTJ::extract, Self::ShowTJ),
            map(ShowTj::extract, Self::ShowTj),
        ))(input)
    }
}

mod parsing;
