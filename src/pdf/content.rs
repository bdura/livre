use winnow::{combinator::trace, BStr, PResult, Parser};

use crate::{
    extraction::{extract, Stream},
    Extract,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentStream(Vec<u8>);

impl Extract<'_> for ContentStream {
    fn extract(input: &mut &BStr) -> PResult<Self> {
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
