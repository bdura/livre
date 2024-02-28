use nom::{
    branch::alt,
    bytes::complete::{tag, take_while, take_while1},
    character::{is_newline, is_space},
    IResult,
};

pub const WHITE_SPACE_CHARS: [u8; 6] = [0x00, 0x09, 0x0A, 0x0C, 0x0D, 0x20];

/// Take a single end-of-line maker.
///
/// Since we're dealing with standards such as PdfEncoding and UTF16BE,
/// of course the end-of-line marker is not just `\n`...
pub fn take_eol(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((tag("\n"), tag("\r\n"), tag("\r")))(input)
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
    alt((tag("\n"), tag("\r\n")))(input)
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

pub fn space(input: &[u8]) -> IResult<&[u8], &[u8]> {
    tag(" ")(input)
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
