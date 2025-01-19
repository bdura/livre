use std::{borrow::Cow, fmt::Debug};

use winnow::{
    combinator::{fail, peek, trace},
    dispatch,
    token::{any, take_till, take_while},
    BStr, PResult, Parser,
};

use crate::extraction::{
    extract,
    utilities::{escaped_sequence, Parentheses},
    Extract,
};

/// Struct that represent a PDF "literal string", ie one represented within
/// parentheses.
///
/// Note that although they are named `Strings` in the PDF specification, [`LiteralString`]s
/// are not necessarily valid UTF-8 and, as such, not represetable by a Rust [`String`].
///
/// No attempt to decode the string is made at this point. Livre represents
/// this data structure as plain bytes (see intro to PDF strings module).
///
/// # Examples
///
/// From the specification:
///
/// ```text
/// (These \
/// two strings \
/// are the same.)
/// (These two strings are the same.)
///
/// (This string has an end-of-line at the end of it.
/// )
/// (So does this one.\n)
///
/// (This string contains \245two octal characters\307.)
/// ```
#[derive(Clone, PartialEq, Eq)]
pub struct LiteralString(pub Vec<u8>);

impl Debug for LiteralString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LiteralString({})", String::from_utf8_lossy(&self.0))?;
        Ok(())
    }
}

impl From<String> for LiteralString {
    fn from(value: String) -> Self {
        Self(value.into_bytes())
    }
}

impl<'a> From<&'a str> for LiteralString {
    fn from(value: &'a str) -> Self {
        Self(value.as_bytes().to_vec())
    }
}

impl<'a> From<Cow<'a, str>> for LiteralString {
    fn from(value: Cow<'a, str>) -> Self {
        Self(value.as_bytes().to_vec())
    }
}

impl<'a> From<Cow<'a, [u8]>> for LiteralString {
    fn from(value: Cow<'a, [u8]>) -> Self {
        Self(value.to_vec())
    }
}

impl Extract<'_> for LiteralString {
    fn extract(input: &mut &BStr) -> PResult<Self> {
        // NOTE: we have to parse the entire sequence first, otherwise we would not
        // know what to do with a closing parenthesis.
        // This contrast with the extraction strategy for `Vec<T>` for instance,
        // since in that case we can just match the opening bracket, keep applying
        // the parser for T, and finally match on the closing bracket.
        let Parentheses(mut inner) = extract(input)?;

        trace(
            "livre-literal-string",
            escaped_sequence(take_till(0.., b'\\'), b'\\'.void(), escape_string).map(Self::from),
        )
        .parse_next(&mut inner)
    }
}

static EMPTY: &[u8] = b"";
static NEWLINE: &[u8] = b"\n";
static RETURN: &[u8] = b"\r";
static TAB: &[u8] = b"\t";
static B: &[u8] = &[33, 161];
static F: &[u8] = &[35, 43];
static LEFT_PAR: &[u8] = b"(";
static RIGHT_PAR: &[u8] = b")";
static BACKSLASH: &[u8] = b"\\";

/// Escape [`LiteralString`] characters.
fn escape_string<'de>(input: &mut &'de BStr) -> PResult<Cow<'de, [u8]>> {
    dispatch! {peek(any);
        b'\n' => any.value(Cow::Borrowed(EMPTY)),
        b'n' => any.value(Cow::Borrowed(NEWLINE)),
        b'r' => any.value(Cow::Borrowed(RETURN)),
        b'\t' => any.value(Cow::Borrowed(TAB)),
        b'b' => any.value(Cow::Borrowed(B)),
        b'f' => any.value(Cow::Borrowed(F)),
        b'(' => any.value(Cow::Borrowed(LEFT_PAR)),
        b')' => any.value(Cow::Borrowed(RIGHT_PAR)),
        b'\\' => any.value(Cow::Borrowed(BACKSLASH)),
        b'0'..b'8' => parse_octal.map(|n| Cow::Owned(vec![n])),
        _ => fail,
    }
    .parse_next(input)
}

/// Parse up to 3 bytes to get the number represented by the underlying octal code.
///
/// NOTE: the PDF specs allow 1 to 3 digits for the octal escape sequence.
/// Contrary to Hexadecimal Strings, missing digits are interpreted as *leading* zeros.
fn parse_octal(input: &mut &BStr) -> PResult<u8> {
    trace("livre-octal", |i: &mut &BStr| {
        let num = take_while(1..=3, b'0'..b'8').parse_next(i)?;

        // SAFETY: `num` only contains octal digits,
        // and is therefore both utf8-encoded and parseable
        let s = unsafe { std::str::from_utf8_unchecked(num) };

        // NOTE: three octal digits may produce an overflow. For instance,
        // `777` is *not* a valid u8 number. In practice, the PDF specs state
        // that "high-order overflow shall be ignored".
        // Besides, a PDF that displays this kind of failure case is probably
        // not worth parsing...
        let n =
            u8::from_str_radix(s, 8).expect("by construction, valid number. should not overflow.");
        Ok(n)
    })
    .parse_next(input)
}

#[cfg(test)]
mod tests {

    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(b"1", 0o1)]
    #[case(b"17", 0o17)]
    #[case(b"137", 0o137)]
    fn octal_parsing(#[case] input: &[u8], #[case] expected: u8) {
        let num = parse_octal(&mut input.as_ref()).unwrap();
        assert_eq!(expected, num);
    }

    #[rstest]
    #[case(b"(abcd)", b"abcd")]
    #[case(b"(test)", b"test")]
    #[case(b"(test\n)", b"test\n")]
    #[case(b"(test (with inner parenthesis))", b"test (with inner parenthesis)")]
    #[case(b"(\\0533)", b"+3")]
    #[case(b"(te\\\\st)", b"te\\st")]
    #[case(b"(te\\\nst)", b"test")]
    fn literal_string(#[case] input: &[u8], #[case] expected: &[u8]) {
        let LiteralString(inner) = extract(&mut input.as_ref()).unwrap();
        assert_eq!(expected, &inner);
    }

    #[rstest]
    #[case("abcd", "LiteralString(abcd)")]
    #[case("test", "LiteralString(test)")]
    fn literal_string_debug<T: Into<LiteralString>>(#[case] input: T, #[case] expected: &str) {
        let result = input.into();
        assert_eq!(format!("{result:?}"), expected);
    }
}
