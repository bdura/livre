use std::collections::HashMap;

use livre_extraction::Extract;
use livre_objects::{Indirect, Reference};
use livre_structure::{crossref::CrossRefs, StartXRef, Trailer, TrailerDict};
use livre_utilities::take_whitespace;
use nom::{
    multi::many0,
    sequence::{preceded, terminated},
    IResult,
};

#[derive(Debug, Clone)]
pub struct Update<'input> {
    pub body: HashMap<Reference, &'input [u8]>,
    pub crossrefs: CrossRefs,
    pub trailer: TrailerDict,
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

        let (input, crossrefs) = preceded(take_whitespace, CrossRefs::extract)(input)?;
        let (input, Trailer(trailer)) = preceded(take_whitespace, Trailer::extract)(input)?;
        let (input, startxref) = preceded(take_whitespace, StartXRef::extract)(input)?;

        let update = Self {
            body,
            crossrefs,
            trailer,
            startxref,
        };

        Ok((input, update))
    }
}
