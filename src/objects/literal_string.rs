use nom::{
    branch::alt,
    bytes::complete::{tag, take},
    combinator::map,
    multi::many0,
    IResult,
};

use crate::utilities::{parse_octal, parse_string_with_escapes, take_within_balanced};

/// Represents a boolean within a PDF.
#[derive(Debug, PartialEq, Clone)]
pub struct LiteralString(pub(crate) String);

impl LiteralString {
    fn escaped_char(input: &[u8]) -> IResult<&[u8], Option<char>> {
        let (input, _) = take(1usize)(input)?;

        alt((
            map(tag(b"\n"), |_| None),
            map(tag(b"n"), |_| Some('\n')),
            map(tag(b"r"), |_| Some('\r')),
            map(tag(b"t"), |_| Some('\t')),
            map(tag(b"b"), |_| Some('\u{21A1}')),
            map(tag(b"f"), |_| Some('\u{232B}')),
            map(tag(b"("), |_| Some('(')),
            map(tag(b")"), |_| Some(')')),
            map(tag(b"\\"), |_| Some('\\')),
            map(parse_octal, |n| Some(n as char)),
        ))(input)
    }

    pub(crate) fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, value) = take_within_balanced(b'(', b')')(input)?;
        let (d, lines) = many0(parse_string_with_escapes(b'\\', Self::escaped_char))(value)?;
        assert!(d.is_empty());
        Ok((input, Self(lines.join(""))))
    }
}

macro_rules! into {
    ($into:ty) => {
        impl From<$into> for LiteralString {
            fn from(value: $into) -> Self {
                let s: String = value.into();
                Self(s)
            }
        }
    };
}

into!(String);
into!(&str);

impl From<LiteralString> for String {
    fn from(value: LiteralString) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn parse(input: &[u8]) -> LiteralString {
        let (_, obj) = LiteralString::parse(input).unwrap();
        obj
    }

    #[allow(clippy::bool_assert_comparison)]
    #[rstest]
    #[case(b"(abcd)", "abcd")]
    #[case(b"(test)", "test")]
    #[case(b"(test\n)", "test\n")]
    #[case(b"(test (with inner parenthesis))", "test (with inner parenthesis)")]
    #[case(b"(\\0533)", "+3")]
    #[case(b"(te\\\\st)", "te\\st")]
    #[case(b"(te\\\nst)", "test")]
    fn integer(#[case] input: &[u8], #[case] result: &str) {
        assert_eq!(parse(input), result.into());

        let result = result.to_string();
        let parsed: String = parse(input).into();
        assert_eq!(parsed, result);
    }
}
