use nom::{
    branch::alt,
    bytes::complete::{tag, take},
    combinator::map,
    multi::many0,
    IResult,
};

use livre_utilities::{parse_octal, parse_string_with_escapes, take_within_balanced};

use crate::extraction::Extract;

impl Extract<'_> for String {
    fn extract(input: &[u8]) -> nom::IResult<&[u8], Self> {
        let (input, value) = take_within_balanced(b'(', b')')(input)?;
        let (d, lines) = many0(parse_string_with_escapes(b'\\', escaped_char))(value)?;
        assert!(d.is_empty());
        Ok((input, lines.join("")))
    }
}

fn escaped_char(input: &[u8]) -> IResult<&[u8], Option<char>> {
    let (input, _) = take(1usize)(input)?;

    alt((
        map(tag("\n"), |_| None),
        map(tag("n"), |_| Some('\n')),
        map(tag("r"), |_| Some('\r')),
        map(tag("t"), |_| Some('\t')),
        map(tag("b"), |_| Some('\u{21A1}')),
        map(tag("f"), |_| Some('\u{232B}')),
        map(tag("("), |_| Some('(')),
        map(tag(")"), |_| Some(')')),
        map(tag("\\"), |_| Some('\\')),
        map(parse_octal, |n| Some(n as char)),
    ))(input)
}

#[cfg(test)]
mod tests {

    use rstest::rstest;

    use super::*;
    use crate::extraction::Parse;

    #[rstest]
    #[case(b"(abcd)", "abcd")]
    #[case(b"(test)", "test")]
    #[case(b"(test\n)", "test\n")]
    #[case(b"(test (with inner parenthesis))", "test (with inner parenthesis)")]
    #[case(b"(\\0533)", "+3")]
    #[case(b"(te\\\\st)", "te\\st")]
    #[case(b"(te\\\nst)", "test")]
    fn string(#[case] input: &[u8], #[case] expected: &str) {
        let (_, parsed) = String::extract(input).unwrap();
        assert_eq!(parsed, expected);
        assert_eq!(input.parse::<String>().unwrap(), expected);
    }
}
