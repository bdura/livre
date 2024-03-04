use std::borrow::Cow;

use livre_utilities::{take_eol_no_r, take_whitespace};
use nom::{
    bytes::complete::{tag, take},
    sequence::tuple,
    IResult,
};

use livre_extraction::{Extract, FromDict, MaybeArray, RawDict};

use livre_filters::{Filter, Filtering, Result};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Stream<'input, T> {
    pub structured: T,
    pub filters: Vec<Filter>,
    pub inner: &'input [u8],
}

impl<'input, T> Stream<'input, T>
where
    T: FromDict<'input>,
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

        let structured = T::from_dict(dict).unwrap();

        let stream = Self {
            structured,
            inner,
            filters,
        };

        Ok((input, stream))
    }
}

impl<'input, T> Extract<'input> for Stream<'input, T>
where
    T: FromDict<'input>,
{
    fn extract(input: &'input [u8]) -> nom::IResult<&'input [u8], Self> {
        let (input, dict) = RawDict::extract(input)?;
        Self::extract_from_dict(input, dict)
    }
}

impl<'input, T> Stream<'input, T> {
    pub fn decode(&self) -> Result<Cow<'input, [u8]>> {
        let mut decoded = Cow::from(self.inner);

        for filter in &self.filters {
            decoded = Cow::Owned(filter.decode(&decoded)?);
        }

        Ok(decoded)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use indoc::indoc;
    use livre_extraction::NoOp;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(
        indoc! {b"
            <</Length 10/Test true>> stream
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
        let (_, stream) = Stream::<'_, HashMap<String, &[u8]>>::extract(input).unwrap();
        assert_eq!(stream.inner, expected);

        assert_eq!(stream.structured.len(), dict.len());

        for (k, v) in dict {
            assert_eq!(stream.structured.get(k).unwrap(), &v);
        }

        // Dummy decode
        assert_eq!(stream.decode().unwrap(), expected);
    }

    #[test]
    fn real_world() {
        let input = &include_bytes!("../../../tests/text.pdf")[0x645D..(0x645D + 410)];
        let (_, stream) = Stream::<'_, NoOp>::extract(input).unwrap();

        assert_eq!(stream.filters.len(), 1);
        assert_eq!(stream.inner.len(), 351);

        stream.decode().unwrap();
    }
}
