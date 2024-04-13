use livre_extraction::{extract, Brackets, Extract};
use livre_utilities::take_whitespace1;
use nom::{
    branch::alt,
    bytes::complete::tag,
    combinator::map,
    multi::many0,
    sequence::{preceded, tuple},
    IResult,
};

/// `Tj` operator: show a text string.
#[derive(Debug, PartialEq, Clone)]
pub struct ShowTj(pub String);

impl Extract<'_> for ShowTj {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        let (input, text) = extract_text(input)?;
        let (input, _) = tuple((take_whitespace1, tag(b"Tj")))(input)?;
        Ok((input, Self(text)))
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
pub struct ShowApostrophe(pub String);

impl Extract<'_> for ShowApostrophe {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        let (input, text) = extract_text(input)?;
        let (input, _) = tuple((take_whitespace1, tag(b"'")))(input)?;
        Ok((input, Self(text)))
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
    pub text: String,
    pub character_spacing: f32,
    pub word_spacing: f32,
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

#[derive(Debug, PartialEq, Clone)]
pub enum ArrayElement {
    Positioning(f32),
    Text(String),
}

impl Extract<'_> for ArrayElement {
    fn extract(input: &'_ [u8]) -> IResult<&'_ [u8], Self> {
        let (input, rest) = alt((
            map(extract_text, ArrayElement::Text),
            map(f32::extract, |p| ArrayElement::Positioning(p / 1000.0)),
        ))(input)?;
        Ok((input, rest))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ShowTJ(pub Vec<ArrayElement>);

impl Extract<'_> for ShowTJ {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        let (input, Brackets(brackets)) = extract(input)?;
        let (input, _) = tuple((take_whitespace1, tag(b"TJ")))(input)?;

        let (_, array) = many0(ArrayElement::extract)(brackets)?;

        Ok((input, Self(array)))
    }
}

/// Extract text (or single character)
fn extract_text(input: &[u8]) -> IResult<&[u8], String> {
    alt((String::extract, map(char::extract, |c| c.into())))(input)
}

#[cfg(test)]
mod tests {
    use livre_extraction::extract;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(b"(test) Tj", "test")]
    #[case(b"<0048> Tj", "H")]
    #[case(b"<0057> Tj", "W")]
    #[case(b"<0052> Tj", "R")]
    fn show_tj(#[case] input: &[u8], #[case] expected: &str) {
        let (_, ShowTj(text)) = extract(input).unwrap();
        assert_eq!(text, expected)
    }

    #[rstest]
    #[case(
        b"[(Bie)7(n)-4( co)-4(rd)-5(iale)-8(m)4(e)4(n)-4(t,)] TJ",
        "Bien cordialement,",
        -0.01,
    )]
    fn show_uc_tj(#[case] input: &[u8], #[case] expected: &str, #[case] offset: f32) {
        let (_, ShowTJ(array)) = extract(input).unwrap();

        let mut text = String::new();
        let mut off = 0.0;

        array.into_iter().for_each(|element| match element {
            ArrayElement::Text(t) => text.push_str(&t),
            ArrayElement::Positioning(p) => off += p,
        });
        assert_eq!(text, expected);
        assert_eq!(off, offset);
    }
}
