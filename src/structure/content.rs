use winnow::{combinator::trace, BStr, ModalResult, Parser};

use crate::extraction::{extract, Extract, Stream};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentStream(Vec<u8>);

impl Extract<'_> for ContentStream {
    fn extract(input: &mut &BStr) -> ModalResult<Self> {
        trace("livre-content-stream", move |i: &mut &BStr| {
            let Stream {
                structured: (),
                content,
            } = extract(i)?;
            Ok(Self(content))
        })
        .parse_next(input)
    }
}
