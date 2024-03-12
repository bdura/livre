use std::collections::HashMap;

use livre_extraction::Extract;
use livre_objects::{Indirect, Reference};
use livre_structure::{StartXRef, Trailer};
use livre_utilities::{take_whitespace, take_whitespace1};
use nom::{
    bytes::complete::tag,
    multi::many0,
    sequence::{preceded, terminated, tuple},
    IResult,
};

#[derive(Debug, Clone)]
pub struct Update<'input> {
    pub body: HashMap<Reference, &'input [u8]>,
    pub trailer: Trailer,
    pub startxref: StartXRef,
}

impl<'input> Extract<'input> for Update<'input> {
    fn extract(input: &'input [u8]) -> IResult<&'input [u8], Self> {
        let (input, indirect_objects) =
            many0(terminated(Indirect::<&[u8]>::extract, take_whitespace))(input)?;

        let body: HashMap<Reference, &[u8]> = indirect_objects
            .into_iter()
            .map(|Indirect { reference, inner }| (reference, inner))
            .collect();

        let (input, trailer) = preceded(take_whitespace, Trailer::extract)(input)?;
        let (input, startxref) = preceded(take_whitespace, StartXRef::extract)(input)?;
        let (input, _) = tuple((take_whitespace1, tag(b"%%EOF"), take_whitespace))(input)?;

        let update = Self {
            body,
            trailer,
            startxref,
        };

        Ok((input, update))
    }
}
