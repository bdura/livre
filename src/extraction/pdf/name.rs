use std::{borrow::Cow, fmt::Debug};

use winnow::{
    ascii::hex_uint,
    combinator::{preceded, trace},
    token::{take, take_till},
    BStr, PResult, Parser,
};

use crate::{extraction::utilities::escaped_sequence, Extract};

#[derive(PartialEq, Clone)]
pub struct Name(pub Vec<u8>);

impl Debug for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = String::from_utf8_lossy(&self.0);
        write!(f, "Name({text})")
    }
}

impl<'de> Extract<'de> for Name {
    fn recognize(input: &mut &'de BStr) -> PResult<&'de [u8]> {
        (b'/', take_till(1.., b"\r\n \t/<>[]("))
            .take()
            .parse_next(input)
    }

    fn extract(input: &mut &BStr) -> PResult<Self> {
        let content = preceded(b'/', take_till(1.., b"\r\n \t/<>[](")).parse_next(input)?;

        escaped_sequence(take_till(0.., b'#'), b'#'.void(), escape_name)
            .map(|name| Self(name.to_vec()))
            .parse_next(&mut content.as_ref())
    }
}

fn escape_name<'de>(input: &mut &'de BStr) -> PResult<Cow<'de, [u8]>> {
    let mut num = take(2usize).parse_next(input)?;
    let n = hex_uint(&mut num)?;

    Ok(Cow::Owned(vec![n]))
}

impl From<Name> for String {
    fn from(Name(value): Name) -> Self {
        String::from_utf8_lossy(&value).to_string()
    }
}

impl From<String> for Name {
    fn from(text: String) -> Self {
        Self(text.into_bytes())
    }
}

impl From<&str> for Name {
    fn from(text: &str) -> Self {
        Self(text.as_bytes().to_vec())
    }
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
    fn name(#[case] input: &[u8], #[case] result: &str) {
        let name = Name::extract(&mut input.as_ref()).unwrap();
        assert_eq!(name, result.into());
    }
}
