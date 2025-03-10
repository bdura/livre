use std::{borrow::Cow, fmt::Debug, ops::Deref};

use winnow::{
    ascii::hex_uint,
    combinator::{preceded, trace},
    error::StrContext,
    token::{take, take_till},
    BStr, PResult, Parser,
};

use crate::extraction::{utilities::escaped_sequence, Extract};

/// In the PDF specification, a *Name* represents a dictionary key,
/// or a (sub-)type specification. It uses an escape scheme using
/// a NUMBER SIGN (`#`) as the escape key.
///
/// The specification defines it as "an atomic symbol uniquely defined
/// by a sequence of any characters (8-bit values)". However, the
/// specification immediately amends that definition by:
///
/// > No token delimiter (such as white-space) occurs between the SOLIDUS
/// > and the encoded name. White-space used as part of a name shall
/// > always be coded using the 2-digit hexadecimal notation.
///
/// In practice, it looks like the characters ` \r\n\t/<>[]>` must be
/// escaped within a name (or at least the opening delimiters), since
/// there would be no way of knowing whether `<` denotes the start of
/// a hex string/dictionary or the continuation of the name object.
#[derive(PartialEq, Hash, Eq, Clone)]
pub struct Name(pub Vec<u8>);

impl Debug for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = String::from_utf8_lossy(&self.0);
        write!(f, "Name({text})")
    }
}

impl<'de> Extract<'de> for Name {
    fn recognize(input: &mut &'de BStr) -> PResult<&'de [u8]> {
        trace(
            "livre-recognize-name",
            (b'/', take_till(1.., b"\r\n \t/<>[](")).take(),
        )
        .parse_next(input)
    }

    fn extract(input: &mut &'de BStr) -> PResult<Self> {
        trace("livre-name", move |i: &mut &'de BStr| {
            let content = preceded(b'/', take_till(1.., b"\r\n \t/<>[](")).parse_next(i)?;

            escaped_sequence(take_till(0.., b'#'), b'#'.void(), escape_name)
                .map(|name| Self(name.to_vec()))
                .parse_next(&mut content.as_ref())
        })
        .context(StrContext::Label("name"))
        .parse_next(input)
    }
}

fn escape_name<'de>(input: &mut &'de BStr) -> PResult<Cow<'de, [u8]>> {
    let mut num = take(2usize).parse_next(input)?;
    let n = hex_uint(&mut num)?;

    Ok(Cow::Owned(vec![n]))
}

impl Deref for Name {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Name> for String {
    fn from(Name(value): Name) -> Self {
        String::from_utf8_lossy(&value).to_string()
    }
}

impl<T> From<T> for Name
where
    T: AsRef<[u8]>,
{
    fn from(value: T) -> Self {
        let value = value.as_ref();
        Self(value.into())
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
        assert_eq!(format!("{name:?}"), format!("Name({result})"));
    }
}
