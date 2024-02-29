use nom::combinator::opt;

use crate::extraction::Extract;

impl<'input, T> Extract<'input> for Option<T>
where
    T: Extract<'input>,
{
    fn extract(input: &'input [u8]) -> nom::IResult<&'input [u8], Self> {
        opt(T::extract)(input)
    }
}

#[cfg(test)]
mod tests {

    use rstest::rstest;

    use super::*;
    use crate::extraction::Parse;

    #[rstest]
    #[case(b"true", Some(true))]
    #[case(b"false", Some(false))]
    #[case(b"", None)]
    #[case(b"12", None)]
    fn opt_bool(#[case] input: &[u8], #[case] expected: Option<bool>) {
        let (_, parsed) = Option::<bool>::extract(input).unwrap();
        assert_eq!(parsed, expected);
        assert_eq!(input.parse::<Option<bool>>().unwrap(), expected);
    }

    #[rstest]
    #[case(b"-23", Some(-23))]
    #[case(b"42", Some(42))]
    #[case(b"", None)]
    #[case(b"true", None)]
    fn opt_i32(#[case] input: &[u8], #[case] expected: Option<i32>) {
        let (_, parsed) = Option::<i32>::extract(input).unwrap();
        assert_eq!(parsed, expected);
        assert_eq!(input.parse::<Option<i32>>().unwrap(), expected);
    }
}
