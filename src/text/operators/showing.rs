use std::borrow::Cow;
use std::fmt::{self, Debug};

use crate::objects::HexBytes;
use crate::parsers::{extract, Brackets, Extract, HexU16};
use crate::parsers::{pdf_decode, take_whitespace1, LitBytes};
use crate::text::TextState;
use itertools::Itertools;
use nom::{
    branch::alt,
    bytes::complete::tag,
    combinator::map,
    multi::many0,
    sequence::{preceded, tuple},
    IResult,
};

use super::Operator;

/// It looks like text can be represented with u8 or u16 code points.
#[derive(PartialEq, Clone)]
pub enum PdfString {
    Utf8(Vec<u8>),
    Utf16(Vec<u16>),
}

impl From<PdfString> for Vec<u8> {
    fn from(value: PdfString) -> Self {
        match value {
            PdfString::Utf8(input) => input,
            PdfString::Utf16(input) => input.into_iter().map(|b| b as u8).collect(),
        }
    }
}

impl Extract<'_> for PdfString {
    fn extract(input: &'_ [u8]) -> IResult<&'_ [u8], Self> {
        extract_text(input)
    }
}

/// `Tj` operator: show a text string.
#[derive(Debug, PartialEq, Clone)]
pub struct ShowTj(pub PdfString);

impl Extract<'_> for ShowTj {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        let (input, text) = extract_text(input)?;
        let (input, _) = tuple((take_whitespace1, tag(b"Tj")))(input)?;
        Ok((input, Self(text)))
    }
}

impl Operator for ShowTj {
    fn apply(self, obj: &mut TextState) {
        let Self(text) = self;
        obj.show_text(text)
    }
}

/// `'` operator: move to the next line and show a text string.
///
/// Equivalent to:
///
/// ```no-rust
/// T*
/// (string) Tj
/// ```
#[derive(Debug, PartialEq, Clone)]
pub struct ShowApostrophe(pub PdfString);

impl Extract<'_> for ShowApostrophe {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        let (input, text) = extract_text(input)?;
        let (input, _) = tuple((take_whitespace1, tag(b"'")))(input)?;
        Ok((input, Self(text)))
    }
}

impl Operator for ShowApostrophe {
    fn apply(self, obj: &mut TextState) {
        let Self(text) = self;
        obj.next_line();
        obj.show_text(text);
    }
}

/// `"` operator: move to the next line and show a text string, using aw as the word
/// spacing and ac as the character spacing (setting the corresponding
/// parameters in the text state). aw and ac shall be numbers expressed in
/// unscaled text space units. This operator shall have the same effect as
/// this code:
///
/// ```no-rust
/// aw Tw
/// ac Tc
/// string '
/// ```
#[derive(Debug, PartialEq, Clone)]
pub struct ShowQuote {
    pub text: PdfString,
    pub character_spacing: f32,
    pub word_spacing: f32,
}

impl Operator for ShowQuote {
    fn apply(self, obj: &mut TextState) {
        let Self {
            text,
            character_spacing,
            word_spacing,
        } = self;

        obj.set_character_spacing(character_spacing);
        obj.set_word_spacing(word_spacing);
        obj.next_line();
        obj.show_text(text);
    }
}

impl Extract<'_> for ShowQuote {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        let (input, (character_spacing, word_spacing)) = extract(input)?;
        let (input, text) = preceded(take_whitespace1, extract_text)(input)?;
        let (input, _) = tuple((take_whitespace1, tag(b"\"")))(input)?;

        let res = Self {
            text,
            character_spacing,
            word_spacing,
        };

        Ok((input, res))
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum ArrayElement {
    Positioning(f32),
    Text(PdfString),
}

impl Debug for PdfString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Utf8(f0) => f
                .debug_tuple("Utf8")
                .field(&String::from_utf8_lossy(f0))
                .finish(),
            Self::Utf16(f0) => f
                .debug_tuple("Utf16")
                .field(&String::from_utf16_lossy(f0))
                .finish(),
        }
    }
}

impl Extract<'_> for ArrayElement {
    fn extract(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, rest) = alt((
            map(extract_text, ArrayElement::Text),
            map(f32::extract, |p| ArrayElement::Positioning(p / 1000.0)),
        ))(input)?;
        Ok((input, rest))
    }
}

impl Operator for ArrayElement {
    fn apply(self, obj: &mut TextState) {
        match self {
            Self::Positioning(amount) => obj.offset_tj(amount),
            Self::Text(text) => obj.show_text(text),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ShowTJ(pub Vec<ArrayElement>);

impl Operator for ShowTJ {
    fn apply(self, obj: &mut TextState) {
        let Self(ops) = self;
        for op in ops {
            obj.apply(op)
        }
    }
}

impl Extract<'_> for ShowTJ {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        let (input, Brackets(brackets)) = extract(input)?;
        let (input, _) = tuple((take_whitespace1, tag(b"TJ")))(input)?;

        let (_, array) = many0(ArrayElement::extract)(brackets)?;

        Ok((input, Self(array)))
    }
}

fn extract_text(input: &[u8]) -> IResult<&[u8], PdfString> {
    alt((
        map(extract_lit, PdfString::Utf8),
        map(extract_hex, PdfString::Utf16),
    ))(input)
}

/// Extract text (or single character)
fn extract_lit(input: &[u8]) -> IResult<&[u8], Vec<u8>> {
    let (input, LitBytes(bytes)) = extract(input)?;
    // TODO: keep borrowing here.
    let res = match bytes {
        Cow::Borrowed(_) => pdf_decode(&bytes).to_vec(),
        Cow::Owned(b) => pdf_decode(&b).to_vec(),
    };
    Ok((input, res))
}

fn extract_hex(input: &[u8]) -> IResult<&[u8], Vec<u16>> {
    let (input, HexU16(vec)) = extract(input)?;
    Ok((input, vec))
}

#[cfg(test)]
mod tests {
    use crate::parsers::extract;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(b"(test) Tj", "test")]
    #[case(b"<0048> Tj", "H")]
    #[case(b"<0057> Tj", "W")]
    #[case(b"<0052> Tj", "R")]
    fn show_tj(#[case] input: &[u8], #[case] expected: &str) {
        let (_, ShowTj(text)) = extract(input).unwrap();
        assert_eq!(Vec::<u8>::from(text), expected.as_bytes())
    }

    #[rstest]
    #[case(
        b"[(Bie)7(n)-4( co)-4(rd)-5(iale)-8(m)4(e)4(n)-4(t,)] TJ",
        "Bien cordialement,",
        -0.01,
    )]
    fn show_uc_tj(#[case] input: &[u8], #[case] expected: &str, #[case] offset: f32) {
        let (_, ShowTJ(array)) = extract(input).unwrap();

        let mut text = Vec::<u8>::new();
        let mut off = 0.0;

        array.into_iter().for_each(|element| match element {
            ArrayElement::Text(t) => text.extend::<&Vec<u8>>(&t.into()),
            ArrayElement::Positioning(p) => off += p,
        });
        assert_eq!(text, expected.as_bytes());
        assert_eq!(off, offset);
    }
}
