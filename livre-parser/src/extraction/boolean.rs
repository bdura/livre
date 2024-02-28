use nom::{branch::alt, bytes::complete::tag};

use super::Extract;

impl Extract<'_> for bool {
    fn extract(input: &[u8]) -> nom::IResult<&[u8], Self> {
        let (input, boolean) = alt((tag("true"), tag("false")))(input)?;
        let boolean = boolean == b"true";
        Ok((input, boolean))
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn boolean() {
        assert!(bool::extract(b"true").unwrap().1);
        assert!(!bool::extract(b"false").unwrap().1);
    }
}
