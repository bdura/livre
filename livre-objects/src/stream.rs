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

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Stream<T> {
    pub decoded: Vec<u8>,
    pub structured: T,
}

impl<'de, T> Extract<'de> for Stream<T>
where
    T: Deserialize<'de>,
{
    fn extract(input: &'de [u8]) -> nom::IResult<&'de [u8], Self> {
        let (
            input,
            StreamDict {
                length,
                filters: MaybeArray(filters),
                structured,
            },
        ) = extract_deserialize(input)?;

        let (input, _) = tuple((take_whitespace, tag(b"stream"), line_ending))(input)?;
        let (input, content) = take(length)(input)?;
        let (input, _) = tuple((take_whitespace, tag("endstream"), take_whitespace))(input)?;

        let decoded = filters.decode(content).map_err(|_| {
            nom::Err::Error(nom::error::Error::from_error_kind(input, ErrorKind::Fail))
        })?;

        let stream = Self {
            structured,
            decoded,
        };

        Ok((input, stream))
    }
}

#[cfg(test)]
mod tests {

    use indoc::indoc;
    use livre_extraction::parse;
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
    fn stream(
        #[case] input: &[u8],
        #[case] expected_stream: &[u8],
        #[case] expected_structured: Test,
    ) {
        let Stream {
            decoded,
            structured,
        } = parse(input).unwrap();

        assert_eq!(expected_stream, decoded);
        assert_eq!(expected_structured, structured);
    }
}
