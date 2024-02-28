use nom::{
    bytes::complete::{tag, take, take_till},
    combinator::opt,
    multi::many0,
    IResult,
};

use crate::utilities::{is_space_or_newline, parse_hexadecimal_bigram, parse_string_with_escapes};

#[derive(Debug, PartialEq, Clone)]
pub struct Name(pub String);

impl Name {
    fn escaped_char(input: &[u8]) -> IResult<&[u8], Option<char>> {
        let (input, _) = take(1usize)(input)?;

        let (input, num) = take(2usize)(input)?;
        let (_, n) = opt(parse_hexadecimal_bigram)(num)?;

        Ok((input, n.map(|n| n as char)))
    }

    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, _) = tag("/")(input)?;
        let (input, value) = take_till(|b| {
            is_space_or_newline(b) || b == b'/' || b == b'<' || b == b'[' || b == b'('
        })(input)?;
        let (d, lines) = many0(parse_string_with_escapes(b'#', Self::escaped_char))(value)?;
        assert!(d.is_empty());
        Ok((input, Self(lines.join(""))))
    }
}

macro_rules! into {
    ($into:ty) => {
        impl From<$into> for Name {
            fn from(value: $into) -> Self {
                let s: String = value.into();
                Self(s)
            }
        }
    };
}

into!(String);
into!(&str);

impl From<Name> for String {
    fn from(value: Name) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn parse(input: &[u8]) -> Name {
        let (_, obj) = Name::parse(input).unwrap();
        obj
    }

    #[allow(clippy::bool_assert_comparison)]
    #[rstest]
    #[case(b"/Name1", "Name1")]
    #[case(b"/ASomewhatLongerName", "ASomewhatLongerName")]
    #[case(
        b"/A;Name_With-Various***Characters?",
        "A;Name_With-Various***Characters?"
    )]
    #[case(b"/1.2", "1.2")]
    #[case(b"/$$", "$$")]
    #[case(b"/@pattern", "@pattern")]
    #[case(b"/.notdef", ".notdef")]
    #[case(b"/Lime#20Green\n", "Lime Green")]
    #[case(b"/paired#28#29parentheses", "paired()parentheses")]
    #[case(b"/The_Key_of_F#23_Minor", "The_Key_of_F#_Minor")]
    #[case(b"/A#42", "AB")]
    fn test_parse(#[case] input: &[u8], #[case] result: &str) {
        assert_eq!(parse(input), result.into());

        let result = result.to_string();
        let parsed: String = parse(input).into();
        assert_eq!(parsed, result);
    }
}
