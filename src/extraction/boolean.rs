use nom::{branch::alt, bytes::complete::tag};

use super::Extract;

impl Extract for bool {
    fn extract(input: &[u8]) -> nom::IResult<&[u8], Self> {
        let (input, boolean) = alt((tag(b"true"), tag(b"false")))(input)?;
        let boolean = boolean == b"true";
        Ok((input, boolean))
    }
}

#[cfg(test)]
mod tests {

    use crate::extraction::Parse;

    use super::*;

    #[allow(clippy::bool_assert_comparison)]
    #[test]
    fn test_parse() {
        assert!(bool::extract(b"true").unwrap().1);
        assert!(!bool::extract(b"false").unwrap().1);

        assert!(b"true".parse::<bool>().unwrap());
        assert!(!b"false".parse::<bool>().unwrap());
    }
}
