use nom::multi::separated_list0;

use livre_utilities::{take_whitespace, take_whitespace1, take_within_balanced};

use crate::extraction::Extract;

impl<'input, T> Extract<'input> for Vec<T>
where
    T: Extract<'input>,
{
    fn extract(input: &'input [u8]) -> nom::IResult<&'input [u8], Self> {
        let (input, value) = take_within_balanced(b'[', b']')(input)?;

        // We need to remove preceding whitespace.
        let (value, _) = take_whitespace(value)?;
        let (value, array) = separated_list0(take_whitespace1, T::extract)(value)?;
        let (value, _) = take_whitespace(value)?;

        assert!(
            value.is_empty(),
            "Remainder is not empty: {:?}",
            String::from_utf8_lossy(value)
        );
        Ok((input, array))
    }
}

#[cfg(test)]
mod tests {

    use rstest::rstest;

    use super::*;
    use crate::extraction::Parse;

    #[rstest]
    #[case(b"[1 2 4]", vec![1, 2, 4])]
    #[case(b"[  1  2 4   ]", vec![1, 2, 4])]
    fn vec_i32(#[case] input: &[u8], #[case] expected: Vec<i32>) {
        let (_, parsed) = Vec::<i32>::extract(input).unwrap();
        assert_eq!(parsed, expected);
        assert_eq!(input.parse::<Vec<i32>>().unwrap(), expected);
    }

    #[rstest]
    #[case(b"[true false]", vec![true, false])]
    #[case(b"[true true    ]", vec![true, true])]
    fn vec_bool(#[case] input: &[u8], #[case] expected: Vec<bool>) {
        let (_, parsed) = Vec::<bool>::extract(input).unwrap();
        assert_eq!(parsed, expected);
    }

    #[rstest]
    #[case(b"[(test)  (teest)]", vec!["test".to_string(), "teest".to_string()])]
    fn vec_string(#[case] input: &[u8], #[case] expected: Vec<String>) {
        let (_, parsed) = Vec::<String>::extract(input).unwrap();
        assert_eq!(parsed, expected);
    }
}
