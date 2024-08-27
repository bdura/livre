use winnow::{
    combinator::{alt, trace},
    BStr, PResult, Parser,
};

use super::Extract;

impl<'de, T> Extract<'de> for Option<T>
where
    T: Extract<'de>,
{
    fn extract(input: &mut &'de BStr) -> PResult<Self> {
        trace(
            "livre-optional",
            alt((T::extract.map(Some), b"null".map(|_| None))),
        )
        .parse_next(input)
    }
}

#[cfg(test)]
mod tests {

    use crate::extraction::extract;
    use rstest::rstest;

    #[rstest]
    #[case(b"true", Some(true))]
    #[case(b"false", Some(false))]
    #[case(b"null", None)]
    fn opt_bool(#[case] input: &[u8], #[case] expected: Option<bool>) {
        let parsed = extract(&mut input.as_ref()).unwrap();
        assert_eq!(expected, parsed);
    }

    #[rstest]
    #[case(b"-23", Some(-23))]
    #[case(b"42", Some(42))]
    #[case(b"null", None)]
    fn opt_i32(#[case] input: &[u8], #[case] expected: Option<i32>) {
        let parsed = extract(&mut input.as_ref()).unwrap();
        assert_eq!(expected, parsed);
    }
}
