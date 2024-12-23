use winnow::combinator::{fail, trace};
use winnow::dispatch;
use winnow::token::any;
use winnow::{BStr, PResult, Parser};

use super::Extract;

/// From the specification:
///
/// > Boolean objects represent the logical values of true and false.
/// > They appear in PDF files using the keywords true and false.
impl Extract<'_> for bool {
    fn extract(input: &mut &BStr) -> PResult<Self> {
        trace(
            "livre-boolean",
            dispatch! {
                any;
                b'f' => b"alse".value(false),
                b't' => b"rue".value(true),
                _ => fail,
            },
        )
        .parse_next(input)
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(b"true", true)]
    #[case(b"false", false)]
    fn boolean<T>(#[case] input: &[u8], #[case] expected: T)
    where
        T: for<'a> Extract<'a> + PartialEq + Debug,
    {
        let result = T::extract(&mut input.as_ref()).unwrap();
        assert_eq!(expected, result);
    }

    #[rstest]
    #[case(b"tru")]
    #[case(b"fals")]
    #[case(b"test")]
    #[should_panic]
    fn wrong_results(#[case] input: &[u8]) {
        bool::extract(&mut input.as_ref()).unwrap();
    }
}
