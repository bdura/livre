use nom::{
    bytes::complete::{tag, take_till},
    sequence::Tuple,
    IResult,
};

use super::utilities::take_whitespace;

pub fn parse_comment(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (input, (_, content, _)) = (
        tag(b"%"),
        take_till(|c| c == b'\n' || c == b'\r'),
        take_whitespace,
    )
        .parse(input)?;

    Ok((input, content))
}

#[derive(Debug)]
pub struct Comment(pub String);

impl Comment {
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, content) = parse_comment(input)?;
        let comment = Self(String::from_utf8(content.to_vec()).unwrap());
        Ok((input, comment))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_comment() {
        let (_, comment) = Comment::parse(b"%this is a test\n").unwrap();
        assert_eq!(comment.0, "this is a test".to_owned())
    }
}
