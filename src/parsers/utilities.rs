use nom::{
    bytes::complete::{take_while, take_while1},
    character::{is_newline, is_space},
    IResult,
};

/// Consumes all whitespace (including newlines).
///
/// # Example
///
/// ```
/// # use pdf_parser::parsers::utilities::take_whitespace;
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
    take_while(|v| is_space(v) || is_newline(v))(input)
}

/// Consumes all whitespace (including newlines, at least one).
///
/// # Example
///
/// ```
/// # use pdf_parser::parsers::utilities::take_whitespace1;
///
/// let (input, ws) = take_whitespace1(b"     test").unwrap();
///
/// assert_eq!(ws.len(), 5);
/// assert_eq!(input, b"test");
///
/// assert!(take_whitespace1(b"test").is_err())
/// ```
pub fn take_whitespace1(input: &[u8]) -> IResult<&[u8], &[u8]> {
    take_while1(|v| is_space(v) || is_newline(v))(input)
}
