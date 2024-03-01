use nom::{
    bytes::complete::{tag, take, take_till},
    combinator::opt,
    multi::many0,
    IResult,
};

use crate::utilities::{
    is_space_or_newline, parse_hexadecimal_bigram, parse_string_with_escapes, take_whitespace,
};

pub(super) fn parse_key_value(input: &[u8]) -> IResult<&[u8], (String, &[u8])> {
    let (input, key) = parse_name(input)?;
    let (input, _) = take_whitespace(input)?;
    let (input, value) = take_till(|b| b == b'/')(input)?;

    Ok((input, (key, value)))
}

fn escaped_char(input: &[u8]) -> IResult<&[u8], Option<char>> {
    let (input, _) = take(1usize)(input)?;

    let (input, num) = take(2usize)(input)?;
    let (_, n) = opt(parse_hexadecimal_bigram)(num)?;

    Ok((input, n.map(|n| n as char)))
}

pub fn parse_name(input: &[u8]) -> IResult<&[u8], String> {
    let (input, _) = tag("/")(input)?;
    let (input, value) =
        take_till(|b| is_space_or_newline(b) || b == b'/' || b == b'<' || b == b'[' || b == b'(')(
            input,
        )?;
    let (d, lines) = many0(parse_string_with_escapes(b'#', escaped_char))(value)?;
    assert!(d.is_empty());
    Ok((input, lines.join("")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

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
    fn key(#[case] input: &[u8], #[case] result: &str) {
        let (_, key) = parse_name(input).unwrap();
        assert_eq!(key, result);
    }

    #[rstest]
    #[case(b"/Name1 (test)", "Name1", b"(test)")]
    #[case(b"/Bool true ", "Bool", b"true ")]
    #[case(b"/Bool true/", "Bool", b"true")]
    fn key_value(#[case] input: &[u8], #[case] key: &str, #[case] value: &[u8]) {
        let (_, (k, v)) = parse_key_value(input).unwrap();
        assert_eq!(k, key);
        assert_eq!(v, value);
    }
}
