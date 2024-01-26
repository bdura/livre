use nom::{
    bytes::complete::{tag, take_till},
    sequence::Tuple,
    IResult,
};

pub fn parse_comment(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (input, (_, content)) =
        (tag(b"%"), take_till(|c| c == b'\n' || c == b'\r')).parse(input)?;

    Ok((input, content))
}

#[derive(Debug)]
pub struct Comment<'a>(pub &'a str);

impl<'a> Comment<'a> {
    pub fn parse(input: &'a [u8]) -> IResult<&'a [u8], Self> {
        let (input, content) = parse_comment(input)?;
        let comment = Self(std::str::from_utf8(content).unwrap());
        // let comment = Self(String::from_utf8_lossy(content).unwrap());
        Ok((input, comment))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_comment() {
        let (_, comment) = Comment::parse(b"%this is a test\n").unwrap();
        assert_eq!(comment.0, "this is a test")
    }
}
