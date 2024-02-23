use std::{fmt::Debug, str::FromStr};

use nom::{
    branch::alt,
    bytes::complete::{tag, take, take_till, take_while, take_while1},
    character::{
        complete::{digit1, oct_digit1},
        is_newline, is_space,
    },
    combinator::{map, opt},
    error::{Error, ErrorKind, ParseError},
    Err, IResult,
};

pub const WHITE_SPACE_CHARS: [u8; 6] = [0x00, 0x09, 0x0A, 0x0C, 0x0D, 0x20];

/// Take a single end-of-line maker.
///
/// Since we're dealing with standards such as PdfEncoding and UTF16BE,
/// of course the end-of-line character is not just `\n`...
pub fn take_eol(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((tag(b"\n"), tag(b"\r\n"), tag(b"\r")))(input)
}

/// Take a single end-of-line maker, excluding a single `\r`.
///
/// According to the specs:
///
/// > an end-of-line marker consisting of either a CARRIAGE RETURN
/// > and a LINE FEED or just a LINE FEED, and not by a CARRIAGE RETURN alone.
///
/// Without this restriction, there would be no way of differentiating between
/// a CRLF and a CR + LF as first byte...
///
/// The moral of the story is that UTF8 with `\n` new line marker is just better.
pub fn take_eol_no_r(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((tag(b"\n"), tag(b"\r\n")))(input)
}

pub fn is_space_or_newline(b: u8) -> bool {
    is_space(b) || is_newline(b) || b == b'\r' || WHITE_SPACE_CHARS.contains(&b)
}

/// Consumes all whitespace (including newlines).
///
/// # Example
///
/// ```
/// # use pdf_parser::utilities::take_whitespace;
///
/// let (input, ws) = take_whitespace(b"     test").unwrap();
///
/// assert_eq!(ws.len(), 5);
/// assert_eq!(input, b"test");
///
/// let (input, ws) = take_whitespace(b"test").unwrap();
///
/// assert_eq!(ws.len(), 0);
/// assert_eq!(input, b"test");
/// ```
pub fn take_whitespace(input: &[u8]) -> IResult<&[u8], &[u8]> {
    take_while(is_space_or_newline)(input)
}

/// Consumes all whitespace (including newlines, at least one).
///
/// # Example
///
/// ```
/// # use pdf_parser::utilities::take_whitespace1;
///
/// let (input, ws) = take_whitespace1(b"     test").unwrap();
///
/// assert_eq!(ws.len(), 5);
/// assert_eq!(input, b"test");
///
/// assert!(take_whitespace1(b"test").is_err())
/// ```
pub fn take_whitespace1(input: &[u8]) -> IResult<&[u8], &[u8]> {
    take_while1(is_space_or_newline)(input)
}

/// Consume the inside of brackets until it is unbalanced.
///
/// Adapted from <https://stackoverflow.com/questions/70630556/parse-allowing-nested-parentheses-in-nom>
pub fn take_within_balanced(
    opening_bracket: u8,
    closing_bracket: u8,
) -> impl Fn(&[u8]) -> IResult<&[u8], &[u8]> {
    move |input: &[u8]| {
        if input[0] != opening_bracket {
            return Err(Err::Error(Error::from_error_kind(
                input,
                ErrorKind::TakeUntil,
            )));
        }

        let mut bracket_counter = 1;

        for (i, &b) in input.iter().enumerate().skip(1) {
            if b == opening_bracket {
                bracket_counter += 1;
            } else if b == closing_bracket {
                bracket_counter -= 1;
            }

            if bracket_counter == 0 {
                return Ok((&input[i + 1..], &input[1..i]));
            }
        }

        Err(Err::Error(Error::from_error_kind(
            input,
            ErrorKind::TakeUntil,
        )))
    }
}

/// Parse up to 3 bytes to get the number represented by the underlying octal code.
pub fn parse_octal(input: &[u8]) -> IResult<&[u8], u8> {
    let value = &input[..input.len().min(3)];
    let (_, num) = oct_digit1(value)?;

    let input = &input[num.len()..];

    let s = unsafe { std::str::from_utf8_unchecked(num) };
    let n = u8::from_str_radix(s, 8).expect("We know it's a valid number.");

    Ok((input, n))
}

pub fn parse_string_with_escapes(
    delimiter: u8,
    parser: impl Fn(&[u8]) -> IResult<&[u8], Option<char>>,
) -> impl Fn(&[u8]) -> IResult<&[u8], String> {
    move |input: &[u8]| {
        if input.is_empty() {
            return Err(Err::Error(Error::from_error_kind(
                input,
                ErrorKind::TakeTill1,
            )));
        }

        let (input, s) = take_till(|b| b == delimiter)(input)?;
        let mut res = String::from_utf8_lossy(s).to_string();

        let (input, modifier) = opt(&parser)(input)?;

        if let Some(m) = Option::flatten(modifier) {
            res.push(m);
        }

        Ok((input, res))
    }
}

pub fn parse_hexadecimal_bigram(input: &[u8]) -> IResult<&[u8], u8> {
    fn inner(input: &[u8]) -> u8 {
        let len = input.len();

        let mut res = {
            // SAFETY: we know for a fact that the supplied input
            // will hold that invariant.
            let num = unsafe { std::str::from_utf8_unchecked(input) };
            u8::from_str_radix(num, 16).unwrap()
        };

        if len == 1 {
            res *= 16;
        }

        res
    }

    alt((map(take(2usize), inner), map(take(1usize), inner)))(input)
}

pub fn parse_digits<O, E>(input: &[u8]) -> IResult<&[u8], O>
where
    O: FromStr<Err = E>,
    E: Debug,
{
    let (input, digits) = digit1(input)?;

    // SAFETY: we know for a fact that `digits` contains digits only,
    // and are therefore both utf-8-encoded and parsable.
    let n = unsafe { std::str::from_utf8_unchecked(digits).parse().unwrap() };

    Ok((input, n))
}

pub fn parse_sign(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((tag(b"+"), tag(b"-")))(input)
}
