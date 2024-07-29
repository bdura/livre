use std::borrow::Cow;

use nom::{
    branch::alt,
    bytes::complete::tag,
    combinator::{map, value},
    IResult,
};

use crate::parsers::{parse_escaped, parse_octal};

use crate::parsers::extraction::{extract, extraction::Extract, Parentheses};

pub struct LitBytes<'input>(pub Cow<'input, [u8]>);

static EMPTY: &[u8] = b"";
static NEWLINE: &[u8] = b"\n";
static RETURN: &[u8] = b"\r";
static TAB: &[u8] = b"\t";
static B: &[u8] = &[33, 161];
static F: &[u8] = &[35, 43];
static LEFT_PAR: &[u8] = b"(";
static RIGHT_PAR: &[u8] = b")";
static BACKSLASH: &[u8] = b"\\";

impl<'input> Extract<'input> for LitBytes<'input> {
    fn extract(input: &'input [u8]) -> nom::IResult<&'input [u8], Self> {
        let (input, Parentheses(value)) = extract(input)?;
        let (_, cow) = parse_escaped(b'\\', escaped_char)(value)?;
        Ok((input, Self(cow)))
    }
}

fn escaped_char(input: &[u8]) -> IResult<&[u8], Cow<'_, [u8]>> {
    alt((
        value(Cow::Borrowed(EMPTY), tag("\n")),
        value(Cow::Borrowed(NEWLINE), tag("n")),
        value(Cow::Borrowed(RETURN), tag("r")),
        value(Cow::Borrowed(TAB), tag("t")),
        value(Cow::Borrowed(B), tag("b")),
        value(Cow::Borrowed(F), tag("f")),
        value(Cow::Borrowed(LEFT_PAR), tag("(")),
        value(Cow::Borrowed(RIGHT_PAR), tag(")")),
        value(Cow::Borrowed(BACKSLASH), tag("\\")),
        map(parse_octal, |n| Cow::Owned(vec![n])),
    ))(input)
}

#[cfg(test)]
mod tests {

    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(b"(abcd)", b"abcd")]
    #[case(b"(test)", b"test")]
    #[case(b"(test\n)", b"test\n")]
    #[case(b"(test (with inner parenthesis))", b"test (with inner parenthesis)")]
    #[case(b"(\\0533)", b"+3")]
    #[case(b"(te\\\\st)", b"te\\st")]
    #[case(b"(te\\\nst)", b"test")]
    fn literal_string(#[case] input: &[u8], #[case] expected: &[u8]) {
        let (_, LitBytes(parsed)) = extract(input).unwrap();
        assert_eq!(parsed, expected);
    }
}
