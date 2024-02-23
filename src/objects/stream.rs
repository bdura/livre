use nom::{
    bytes::complete::{tag, take},
    combinator::cut,
    IResult,
};

use crate::{
    error::Result,
    filters::{Filter, Filtering},
    utilities::take_eol,
};

use super::{array::Array, name::Name, Dictionary, Object};

/// Represents a boolean within a PDF.
#[derive(Debug, PartialEq, Clone)]
pub struct Stream {
    pub(crate) stream: Vec<u8>,
    pub(crate) filters: Vec<Filter>,
}

impl Stream {
    fn parse_stream_body(input: &[u8], length: usize) -> IResult<&[u8], &[u8]> {
        let (input, body) = take(length)(input)?;

        let (input, _) = take_eol(input)?;
        let (input, _) = tag(b"endstream")(input)?;

        Ok((input, body))
    }

    pub(crate) fn parse_with_dict(input: &[u8], dict: Dictionary) -> IResult<&[u8], Self> {
        let Dictionary(dict) = dict;
        let length: usize = dict
            .get("Length")
            .expect("`Length` is a required field in a stream dictionnary.")
            .clone()
            .try_into()
            .expect("`Length` key has to be usize-compatible.");

        // Cut to commit to this branch
        let (input, body) = cut(move |i| Self::parse_stream_body(i, length))(input)?;

        let mut filters = Vec::new();

        match dict.get("Filter") {
            None => {}
            Some(Object::Name(Name(n))) => filters.push(Filter::from_name(n)),
            Some(Object::Array(Array(a))) => {
                for f in a {
                    let Object::Name(Name(n)) = f else {
                        unreachable!("Per the specs, it MUST be an array of names.")
                    };
                    filters.push(Filter::from_name(n))
                }
            }
            _ => unreachable!(
                "Per the specs, the `Filter` key must be empty, a name of an array of names."
            ),
        }

        let stream = body.to_owned();

        let stream = Self { stream, filters };

        Ok((input, stream))
    }
}

impl Stream {
    pub fn decode(&self) -> Result<Vec<u8>> {
        let mut stream = self.stream.clone();
        for filter in &self.filters {
            stream = filter.decode(&stream)?;
        }
        Ok(stream)
    }
}
