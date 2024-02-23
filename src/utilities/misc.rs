use nom::{
    bytes::complete::{tag, take_till},
    combinator::opt,
    error::{Error, ErrorKind, ParseError},
    sequence::tuple,
    Err, IResult,
};

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

pub fn parse_comment(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (input, (_, content)) = tuple((tag(b"%"), take_till(|c| c == b'\n' || c == b'\r')))(input)?;
    Ok((input, content))
}
