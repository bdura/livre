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

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use rstest::rstest;
    use winnow::{
        combinator::{dispatch, fail, peek},
        error::ContextError,
        token::{any, take_till},
        BStr, Parser,
    };

    use super::escaped_sequence;

    /// Escape transform for tests: `n` → `\n`, `t` → `\t`, `\\` → `\\`.
    fn test_transform<'de>(input: &mut &'de BStr) -> winnow::PResult<Cow<'de, [u8]>> {
        static NEWLINE: &[u8] = b"\n";
        static TAB: &[u8] = b"\t";
        static BACKSLASH: &[u8] = b"\\";
        dispatch! { peek(any);
            b'n'  => any.value(Cow::Borrowed(NEWLINE)),
            b't'  => any.value(Cow::Borrowed(TAB)),
            b'\\' => any.value(Cow::Borrowed(BACKSLASH)),
            _     => fail,
        }
        .parse_next(input)
    }

    fn parser<'de>(
    ) -> impl Parser<&'de BStr, Cow<'de, [u8]>, ContextError> {
        escaped_sequence(take_till(0.., b'\\'), b'\\'.void(), test_transform)
    }

    // When there are no escape sequences the normal parser consumes the whole input and the
    // result can borrow directly from it — no allocation needed.
    #[rstest]
    #[case(b"hello")]
    #[case(b"")]
    #[case(b"no backslash here")]
    fn no_escapes_returns_borrowed(#[case] input: &[u8]) {
        let result = parser().parse(input.into()).unwrap();
        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(result.as_ref(), input);
    }

    // When at least one escape sequence is present the output must be an owned allocation.
    #[rstest]
    #[case(b"he\\nllo",  b"he\nllo" as &[u8])]
    #[case(b"\\n",       b"\n")]
    #[case(b"\\t",       b"\t")]
    #[case(b"a\\\\b",    b"a\\b")]
    #[case(b"a\\nb\\tc", b"a\nb\tc")]
    #[case(b"\\na",      b"\na")]
    fn escapes_produce_correct_output(#[case] input: &[u8], #[case] expected: &[u8]) {
        let result = parser().parse(input.into()).unwrap();
        assert_eq!(result.as_ref(), expected);
    }

    // When the transform parser cannot handle the byte after `\`, the whole parser must fail
    // rather than silently consuming or ignoring the bad escape.
    #[test]
    fn unknown_escape_fails() {
        let result = parser().parse(b"abc\\zdef".as_ref().into());
        assert!(result.is_err());
    }
}
