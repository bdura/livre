use winnow::{
    ascii::{line_ending, multispace0, multispace1, till_line_ending},
    combinator::{alt, delimited, repeat, trace},
    BStr, PResult, Parser,
};

/// A comment extractor. This parser returns the line after the `%` sign.
fn comment<'de>(input: &mut &'de BStr) -> PResult<&'de [u8]> {
    trace(
        "livre-comment",
        delimited((b'%', multispace0), till_line_ending, line_ending),
    )
    .parse_next(input)
}

/// Parses 0 or more comment/multispace
pub fn multicomment0<'de>(input: &mut &'de BStr) -> PResult<&'de [u8]> {
    trace(
        "livre-comments-whitespace",
        repeat::<_, _, (), _, _>(0.., alt((comment, multispace1))).take(),
    )
    .parse_next(input)
}

/// Parses at least one comment/multispace
pub fn multicomment1<'de>(input: &mut &'de BStr) -> PResult<&'de [u8]> {
    trace(
        "livre-comments-whitespace",
        repeat::<_, _, (), _, _>(1.., alt((comment, multispace1))).take(),
    )
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(b"% 42\n", b"42")]
    #[case(b"% Another comment\r\n", b"Another comment")]
    fn extract_comment(#[case] input: &[u8], #[case] expected: &[u8]) {
        let mut input = input.as_ref();
        let c = comment(&mut input).unwrap();
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
