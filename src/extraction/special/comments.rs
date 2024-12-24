use winnow::{
    ascii::{line_ending, multispace1, till_line_ending},
    combinator::{alt, delimited, repeat, trace},
    token::take_while,
    BStr, PResult, Parser,
};

use crate::extraction::Extract;

/// PDF comment, which references into the original data.
///
/// The content is borrowed, starts after the first leading `%` and strips leading spaces.
pub struct Comment<'de>(&'de [u8]);

impl<'de> Extract<'de> for Comment<'de> {
    fn extract(input: &mut &'de BStr) -> PResult<Self> {
        trace(
            "livre-comment",
            delimited(
                (b'%', take_while(0.., (b' ', b'\t'))),
                till_line_ending,
                line_ending,
            )
            .map(Comment),
        )
        .parse_next(input)
    }

    fn recognize(input: &mut &'de BStr) -> PResult<&'de [u8]> {
        (b'%', till_line_ending, line_ending)
            .take()
            .parse_next(input)
    }
}

/// Parses 0 or more comment/multispace
pub fn multicomment0<'de>(input: &mut &'de BStr) -> PResult<&'de [u8]> {
    trace(
        "livre-comments-whitespace",
        repeat::<_, _, (), _, _>(0.., alt((Comment::recognize, multispace1))).take(),
    )
    .parse_next(input)
}

/// Parses at least one comment/multispace
pub fn multicomment1<'de>(input: &mut &'de BStr) -> PResult<&'de [u8]> {
    trace(
        "livre-comments-whitespace",
        repeat::<_, _, (), _, _>(1.., alt((Comment::recognize, multispace1))).take(),
    )
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::extraction::extract;

    use super::*;

    #[rstest]
    #[case(b"% 42\n", b"42")]
    #[case(b"% Another comment\r\n", b"Another comment")]
    fn extract_comment(#[case] input: &[u8], #[case] expected: &[u8]) {
        let mut input = input.as_ref();
        let Comment(c) = extract(&mut input).unwrap();
        assert!(input.is_empty(), "{:?}", input);
        assert_eq!(expected, c);
    }

    #[rstest]
    #[case(b"% 42\n")]
    #[case(b"% Another comment\r\n")]
    #[case(b"\r\n")]
    #[case(b"")]
    fn extract_multicomment0(#[case] input: &[u8]) {
        let mut input = input.as_ref();
        multicomment0(&mut input).unwrap();
        assert!(input.is_empty(), "{:?}", input);
    }

    #[rstest]
    #[case(b"% 42\n")]
    #[case(b"% Another comment\r\n")]
    #[case(b"\r\n")]
    fn extract_multicomment1(#[case] input: &[u8]) {
        let mut input = input.as_ref();
        multicomment1(&mut input).unwrap();
        assert!(input.is_empty(), "{:?}", input);
    }

    #[rstest]
    #[case(b"")]
    #[should_panic]
    fn failure_case(#[case] input: &[u8]) {
        let mut input = input.as_ref();
        multicomment1(&mut input).unwrap();
    }
}
