use std::collections::HashMap;

use livre_utilities::{take_eol_no_r, take_whitespace};
use nom::{
    bytes::complete::{tag, take},
    sequence::tuple,
    IResult,
};

use livre_extraction::{Extract, MaybeArray, RawDict};

use livre_filters::Filter;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Stream<'input, T> {
    pub dict: HashMap<String, T>,
    pub filters: Vec<Filter>,
    pub inner: &'input [u8],
}

impl<'input, T> Stream<'input, T>
where
    T: Extract<'input>,
{
    pub(crate) fn extract_from_dict(
        input: &'input [u8],
        mut dict: RawDict<'input>,
    ) -> IResult<&'input [u8], Self> {
        let (input, _) = tuple((take_whitespace, tag(b"stream"), take_eol_no_r))(input)?;

        let length: usize = dict
            .pop("Length")
            .expect("A stream must have a length parameter");

        let MaybeArray(filters) = dict
            .pop_opt("Filter")
            .expect("Filter is a name or array.")
            .unwrap_or_default();

        let (input, inner) = take(length)(input)?;

        let (input, _) = tuple((take_whitespace, tag(b"endstream"), take_whitespace))(input)?;

        let dict = dict.convert().unwrap();

        let stream = Self {
            dict,
            inner,
            filters,
        };

        Ok((input, stream))
    }
}

impl<'input, T> Extract<'input> for Stream<'input, T>
where
    T: Extract<'input>,
{
    fn extract(input: &'input [u8]) -> nom::IResult<&'input [u8], Self> {
        let (input, dict) = RawDict::extract(input)?;
        Self::extract_from_dict(input, dict)
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(
        indoc! {b"
            <</Length 10/Test true/Filter/FlateDecode>> stream
            0123456789
            endstream
        "},
        b"0123456789",
        vec![("Test", b"true".as_slice())]
    )]
    #[case(
        indoc! {b"
            <</Length 1/Test false>>stream
            0
            endstream
        "},
        b"0",
        vec![("Test", b"false".as_slice())]
    )]
    #[case(
        b"<</Length 1/Test false/Ok true>>stream\n0\nendstream",
        b"0",
        vec![("Test", b"false".as_slice()), ("Ok", b"true".as_slice())]
    )]
    fn stream(#[case] input: &[u8], #[case] expected: &[u8], #[case] dict: Vec<(&str, &[u8])>) {
        let (_, stream) = Stream::<'_, &[u8]>::extract(input).unwrap();
        assert_eq!(stream.inner, expected);

        assert_eq!(stream.dict.len(), dict.len());

        for (k, v) in dict {
            assert_eq!(stream.dict.get(k).unwrap(), &v);
        }
    }
}
