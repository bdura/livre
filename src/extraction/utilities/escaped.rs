use std::borrow::Cow;

use winnow::{combinator::fail, error::ContextError, BStr, PResult, Parser};

/// Parse an escaped sequence.
pub fn escaped_sequence<'de, N, E, T>(
    normal: N,
    escaped: E,
    transform: T,
) -> EscapedSequenceParser<N, E, T>
where
    N: Parser<&'de BStr, &'de [u8], ContextError>,
    E: Parser<&'de BStr, (), ContextError>,
    T: Parser<&'de BStr, Cow<'de, [u8]>, ContextError>,
{
    EscapedSequenceParser {
        normal,
        escaped,
        transform,
    }
}

pub struct EscapedSequenceParser<N, E, T> {
    normal: N,
    escaped: E,
    transform: T,
}

impl<'de, N, E, T> Parser<&'de BStr, Cow<'de, [u8]>, ContextError>
    for EscapedSequenceParser<N, E, T>
where
    N: Parser<&'de BStr, &'de [u8], ContextError>,
    E: Parser<&'de BStr, (), ContextError>,
    T: Parser<&'de BStr, Cow<'de, [u8]>, ContextError>,
{
    fn parse_next(&mut self, input: &mut &'de BStr) -> PResult<Cow<'de, [u8]>> {
        let n = input.len();

        let normal = self.normal.parse_next(input)?;

        if input.is_empty() {
            // If we consumed the whole input, then we can just reference into it.
            return Ok(Cow::Borrowed(normal));
        }

        let mut result = {
            let mut v = Vec::with_capacity(n);
            v.extend(normal);
            v
        };

        while !input.is_empty() {
            let n = input.len();

            self.escaped.parse_next(input)?;
            let transform = self.transform.parse_next(input)?;
            result.extend(transform.as_ref());

            let normal = self.normal.parse_next(input)?;
            result.extend(normal);

            if input.len() == n {
                fail.parse_next(input)?;
            }
        }
        Ok(Cow::Owned(result))
    }
}

// TODO: add proper testing for this. For now we only have integration tests through the literal
// string parser and the likes.
