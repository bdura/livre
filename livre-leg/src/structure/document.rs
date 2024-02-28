use std::collections::HashMap;

use nom::{multi::many1, IResult};

use crate::{
    objects::{Object, Reference},
    utilities::take_whitespace,
};

use super::{CrossRefs, Header, StartXRef, Update};

#[derive(Debug, Clone)]
pub struct Document {
    pub header: Header,
    pub body: HashMap<Reference, Object>,
    pub crossrefs: CrossRefs,
    pub root: Reference,
    pub startxref: StartXRef,
}

impl Document {
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, header) = Header::parse(input)?;
        let (input, _) = take_whitespace(input)?;
        let (input, updates) = many1(Update::parse)(input)?;

        let last_update = updates
            .last()
            .expect("There should be at least one update.");

        let root = last_update.trailer.root;
        let startxref = last_update.startxref;

        let mut body = Vec::new();
        let mut crossrefs = Vec::new();

        for update in updates.into_iter().rev() {
            let Update {
                body: b,
                crossrefs: c,
                ..
            } = update;
            body.push(b);
            crossrefs.push(c)
        }

        let doc = Self {
            header,
            body: body.into_iter().flat_map(|h| h.into_iter()).collect(),
            crossrefs: crossrefs
                .into_iter()
                .fold(CrossRefs::default(), |a, b| a.merge(b)),
            root,
            startxref,
        };

        Ok((input, doc))
    }
}
