mod de;
mod error;

use error::{Error, Result};

pub use de::{from_bytes, from_bytes_prefix, from_str, Deserializer};
use nom::{error::ParseError, IResult};
use serde::Deserialize;

pub fn extract_deserialize<'de, T>(input: &'de [u8]) -> IResult<&'de [u8], T>
where
    T: Deserialize<'de>,
{
    from_bytes_prefix(input).map_err(|_| {
        nom::Err::Error(nom::error::Error::from_error_kind(
            input,
            nom::error::ErrorKind::Fail,
        ))
    })
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use rstest::rstest;

    use super::*;

    #[derive(Debug, PartialEq, Deserialize)]
    struct Test {
        int: i32,
        seq: Vec<(bool, u16)>,
    }

    #[rstest]
    #[case(b"12", 12)]
    #[case(b"-3", -3.0)]
    #[case(b"null", None::<i32>)]
    #[case(b"<</int 42 /test 0 1 R/seq[true 1 false 2]>>", Test{ int: 42, seq: vec![(true, 1), (false, 2)] })]
    fn deserialize<'de, T>(#[case] input: &'de [u8], #[case] expected: T)
    where
        T: Deserialize<'de> + PartialEq + Debug,
    {
        let (_, result) = extract_deserialize(input).unwrap();
        assert_eq!(expected, result);
    }
}
