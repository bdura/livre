use std::borrow::Cow;

use nom::{
    bytes::complete::{tag, take, take_till},
    combinator::opt,
    error::{Error, ErrorKind, ParseError},
    sequence::tuple,
    Err, IResult,
};

pub fn parse_string_with_escapes(
    escape: u8,
    parser: impl Fn(&[u8]) -> IResult<&[u8], Option<char>>,
) -> impl Fn(&[u8]) -> IResult<&[u8], String> {
    move |input: &[u8]| {
        if input.is_empty() {
            return Err(Err::Error(Error::from_error_kind(
                input,
                ErrorKind::TakeTill1,
            )));
        }

        let (input, s) = take_till(|b| b == escape)(input)?;
        let mut res = String::from_utf8_lossy(s).to_string();

        let (input, modifier) = opt(&parser)(input)?;

        if let Some(m) = Option::flatten(modifier) {
            res.push(m);
        }

        Ok((input, res))
    }
}

pub fn parse_escaped(
    escape: u8,
    parser: impl for<'a> Fn(&'a [u8]) -> IResult<&'a [u8], Cow<'a, [u8]>>,
) -> impl for<'a> Fn(&'a [u8]) -> IResult<&'a [u8], Cow<'a, [u8]>> {
    move |input: &[u8]| {
        let n = input.len();

        if input.is_empty() {
            return Err(Err::Error(Error::from_error_kind(
                input,
                ErrorKind::TakeTill1,
            )));
        }

        let (mut input, s) = take_till(|b| b == escape)(input)?;

        if input.is_empty() {
            return Ok((input, Cow::Borrowed(s)));
        }

        let mut res = Vec::with_capacity(n);
        res.extend(s);

        while !input.is_empty() {
            let (ni, _) = take(1usize)(input)?;
            let (ni, escaped) = (parser)(ni)?;

            res.extend(escaped.iter());

            let (ni, s) = take_till(|b| b == escape)(ni)?;
            input = ni;
            res.extend(s);
        }

        Ok((input, Cow::Owned(res)))
    }
}

pub fn parse_comment(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (input, (_, content)) = tuple((tag("%"), take_till(|c| c == b'\n' || c == b'\r')))(input)?;
    Ok((input, content))
}
