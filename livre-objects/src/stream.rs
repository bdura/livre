use std::{fmt::Debug, ops::Deref};

use livre_extraction::Extract;
use livre_filters::{Filter, Filtering};
use livre_serde::{extract_deserialize, MaybeArray};
use livre_utilities::take_whitespace;
use nom::{
    bytes::complete::{tag, take},
    character::complete::line_ending,
    error::{ErrorKind, ParseError},
    sequence::tuple,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct StreamDict<T> {
    length: usize,
    #[serde(default)]
    filters: MaybeArray<Filter>,
    #[serde(flatten)]
    structured: T,
}

#[derive(PartialEq, Eq, Clone)]
pub struct Bytes(pub Vec<u8>);

impl<T> From<T> for Bytes
where
    T: Into<Vec<u8>>,
{
    fn from(value: T) -> Self {
        Bytes(value.into())
    }
}

impl Debug for Bytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Bytes")
            .field(&String::from_utf8_lossy(&self.0))
            .finish()
    }
}

impl Deref for Bytes {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Stream<T> {
    pub decoded: Bytes,
    pub structured: T,
}

impl<'de, T> Extract<'de> for Stream<T>
where
    T: Deserialize<'de> + Debug,
{
    fn extract(input: &'de [u8]) -> nom::IResult<&'de [u8], Self> {
        dbg!(Bytes(input.into()));

        let (
            input,
            StreamDict {
                length,
                filters: MaybeArray(filters),
                structured,
            },
        ) = extract_deserialize(input)?;

        dbg!(&structured);

        let (input, _) = tuple((take_whitespace, tag(b"stream"), line_ending))(input)?;
        let (input, content) = take(length)(input)?;
        let (input, _) = tuple((take_whitespace, tag("endstream"), take_whitespace))(input)?;

        let decoded = filters.decode(content).map_err(|_| {
            nom::Err::Error(nom::error::Error::from_error_kind(input, ErrorKind::Fail))
        })?;

        let stream = Self {
            structured,
            decoded: decoded.into(),
        };

        Ok((input, stream))
    }
}

#[cfg(test)]
mod tests {

    use std::collections::HashMap;

    use indoc::indoc;
    use livre_extraction::{extract, parse};
    use rstest::rstest;

    use super::*;

    #[derive(Deserialize, PartialEq, Debug)]
    #[serde(rename_all = "PascalCase")]
    struct Test {
        test: bool,
    }

    #[rstest]
    #[case(
        indoc! {b"
            <</Length 10/Test true>> stream
            0123456789
            endstream
        "},
        b"0123456789",
        Test{test: true},
    )]
    #[case(
        indoc! {b"
            <</Length 1/Test false /Test2 false>>stream
            0
            endstream
        "},
        b"0",
        Test{test: false},
    )]
    #[case(
        b"<</Length 1/Test true/Test2 false>>stream\n0\nendstream",
        b"0",
        Test{test: true},
    )]
    #[case(
        b"<</Length 10/Test/Test>> stream\n0123456789\nendstream\n",
        b"0123456789",
        (),
    )]
    #[case(
        b"<</Length 10/Test/Test>> stream\n0123456789\nendstream\n",
        b"0123456789",
        HashMap::from([("Test".to_string(), "Test".to_string())]),
    )]
    fn stream<'de, T>(
        #[case] input: &'de [u8],
        #[case] expected_stream: &'static [u8],
        #[case] expected_structured: T,
    ) where
        T: Deserialize<'de> + Debug + PartialEq,
    {
        let Stream {
            decoded,
            structured,
        } = parse(input).unwrap();

        assert_eq!(expected_stream, decoded.0);
        assert_eq!(expected_structured, structured);
    }

    #[rstest]
    #[case(
        b"<</Length 10/Test 1>> stream\n0123456789\nendstream\n",
        Stream { decoded: b"0123456789".into(), structured: vec![("Test".into(), 1)].into_iter().collect::<HashMap<String, u8>>() }
    )]
    #[case(
        b"<</Length 10/Test/Test>> stream\n0123456789\nendstream\n",
        Stream { decoded: b"0123456789".into(), structured: vec![("Test".into(), "Test".into())].into_iter().collect::<HashMap<String, String>>() }
    )]
    fn stream2<'de, T>(#[case] input: &'de [u8], #[case] expected: Stream<HashMap<String, T>>)
    where
        T: Deserialize<'de> + Debug + PartialEq,
    {
        let (_, result) = extract(input).unwrap();
        assert_eq!(expected, result);
    }
}
