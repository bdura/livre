mod state;
use enum_dispatch::enum_dispatch;
use nom::{branch::alt, combinator::map};
pub use state::{CharSpace, FontSize, HorizontalScale, Leading, RenderMode, Rise, WordSpace};

mod position;
pub use position::{MoveNextLine, MoveTD, MoveTd, TextMatrix};

mod showing;
pub use showing::{ShowApostrophe, ShowQuote, ShowTJ, ShowTj};

mod other;
pub use other::{Gs, LowercaseG, LowercaseRG, UppercaseG, UppercaseRG};

use crate::parsers::Extract;

use super::TextState;

#[enum_dispatch]
pub trait Operator {
    fn apply(self, obj: &mut TextState);
}

#[enum_dispatch(Operator)]
#[derive(Debug, PartialEq)]
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
    UppercaseG,
    LowercaseG,
    Gs,
    LowercaseRG,
    UppercaseRG,
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
            map(UppercaseG::extract, Self::UppercaseG),
            map(LowercaseG::extract, Self::LowercaseG),
            map(Gs::extract, Self::Gs),
            map(LowercaseRG::extract, Self::LowercaseRG),
            map(UppercaseRG::extract, Self::UppercaseRG),
        ))(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    use crate::parsers::parse;

    #[rstest]
    #[case(b"1.0 1.0 1 RG", UppercaseRG)]
    #[case(b"1.0 1.0 1 rg", LowercaseRG)]
    #[case(b"1.0 g", LowercaseG)]
    #[case(b"/Test gs", Gs)]
    fn op(#[case] input: &[u8], #[case] expected: impl Into<Op>) {
        let result = parse(input).unwrap();
        let expected = expected.into();
        assert_eq!(expected, result);
    }
}
