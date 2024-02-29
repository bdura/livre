use nom::{
    bytes::complete::{tag, take},
    sequence::tuple,
};

use crate::{
    utilities::{take_eol_no_r, take_whitespace},
    Extract,
};

use super::RawDict;

pub struct Stream<'input> {
    pub dict: RawDict<'input>,
    pub inner: &'input [u8],
}

impl<'input> Extract<'input> for Stream<'input> {
    fn extract(input: &'input [u8]) -> nom::IResult<&'input [u8], Self> {
        let (input, mut dict) = RawDict::extract(input)?;
        let (input, _) = tuple((take_whitespace, tag(b"stream"), take_eol_no_r))(input)?;

        let length: usize = dict
            .pop("Length")
            .expect("A stream must have a length parameter");

        let (input, inner) = take(length)(input)?;

        let (input, _) = tuple((take_whitespace, tag(b"endstream"), take_whitespace))(input)?;

        let stream = Self { dict, inner };

        Ok((input, stream))
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(
        b"<</Length 10/Test true>>stream\n0123456789\nendstream",
        b"0123456789",
        vec![("Test", b"true".as_slice())]
    )]
    #[case(
        b"<</Length 1/Test false>>stream\n0\nendstream",
        b"0",
        vec![("Test", b"false".as_slice())]
    )]
    #[case(
        b"<</Length 1/Test false/Ok true>>stream\n0\nendstream",
        b"0",
        vec![("Test", b"false".as_slice()), ("Ok", b"true".as_slice())]
    )]
    fn stream(#[case] input: &[u8], #[case] expected: &[u8], #[case] dict: Vec<(&str, &[u8])>) {
        let (_, stream) = Stream::extract(input).unwrap();
        assert_eq!(stream.inner, expected);

        assert_eq!(stream.dict.len(), dict.len());

        for (k, v) in dict {
            assert_eq!(stream.dict.get(k).unwrap(), &v);
        }
    }
}
