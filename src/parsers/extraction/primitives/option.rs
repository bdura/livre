use nom::{branch::alt, bytes::complete::tag, combinator::map};

use super::Extract;

impl<'input, T> Extract<'input> for Option<T>
where
    T: Extract<'input>,
{
    fn extract(input: &'input [u8]) -> nom::IResult<&'input [u8], Self> {
        alt((map(tag("null"), |_| None), map(T::extract, Some)))(input)
    }
}

#[cfg(test)]
mod tests {

    use rstest::rstest;

    use super::*;
    use crate::parsers::extraction::parse;

    #[rstest]
    #[case(b"true", Some(true))]
    #[case(b"false", Some(false))]
    #[case(b"null", None)]
    fn opt_bool(#[case] input: &[u8], #[case] expected: Option<bool>) {
        let (_, parsed) = Option::<bool>::extract(input).unwrap();
        assert_eq!(parsed, expected);
        assert_eq!(parse::<Option<bool>>(input).unwrap(), expected);
    }

    #[rstest]
    #[case(b"-23", Some(-23))]
    #[case(b"42", Some(42))]
    #[case(b"null", None)]
    fn opt_i32(#[case] input: &[u8], #[case] expected: Option<i32>) {
        let (_, parsed) = Option::<i32>::extract(input).unwrap();
        assert_eq!(parsed, expected);
        assert_eq!(parse::<Option<i32>>(input).unwrap(), expected);
    }
}
