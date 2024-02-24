use std::collections::HashMap;

use nom::{multi::many0, sequence::preceded, IResult};

use crate::{
    objects::{Object, Reference},
    utilities::take_whitespace,
};

use super::{CrossRefs, Trailer};

#[derive(Debug, Clone)]
pub struct Update {
    pub body: HashMap<Reference, Object>,
    pub crossrefs: CrossRefs,
    pub trailer: Trailer,
    // pub startxref: Option<usize>,
}

impl Update {
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, indirect_objects) = many0(Object::parse_indirect)(input)?;
        let body: HashMap<Reference, Object> = indirect_objects.into_iter().collect();

        let (input, crossrefs) = preceded(take_whitespace, CrossRefs::parse)(input)?;
        let (input, trailer) = preceded(take_whitespace, Trailer::parse)(input)?;

        let update = Self {
            body,
            crossrefs,
            trailer,
        };

        Ok((input, update))
    }
}
