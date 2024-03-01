use nom::IResult;

use crate::Extract;

/// The extractor for `&[u8]` just consumes the entire the stream.
/// It's the responsibility of the caller to use it with a well-defined
/// input.
impl<'input> Extract<'input> for &'input [u8] {
    fn extract(input: &'input [u8]) -> IResult<&'input [u8], Self> {
        Ok((b"", input))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(b"[1 2 3]")]
    #[case(b"1")]
    fn raw(#[case] input: &[u8]) {
        let (r, raw) = <&[u8]>::extract(input).unwrap();
        assert_eq!(raw, input);
        assert!(r.is_empty());
    }
}
