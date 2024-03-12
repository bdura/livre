use std::collections::HashMap;

use livre_extraction::{Extract, TypedReference};
use nom::{multi::many1, IResult};

use livre_objects::Reference;

use livre_utilities::take_whitespace;

use livre_structure::{Catalogue, RefLocation, StartXRef, Trailer};

use crate::{Header, Update};

#[derive(Debug, Clone)]
pub struct Document<'input> {
    pub header: Header,
    pub body: HashMap<Reference, &'input [u8]>,
    pub crossrefs: HashMap<Reference, RefLocation>,
    pub root: TypedReference<Catalogue>,
    pub startxref: StartXRef,
}

impl Document<'_> {
    pub fn get_location(&self, reference: impl Into<Reference>) -> Option<RefLocation> {
        let reference = reference.into();
        self.crossrefs.get(&reference).copied()
    }
    pub fn get_referenced_bytes(&self, reference: impl Into<Reference>) -> Option<&[u8]> {
        let reference = reference.into();
        self.body.get(&reference).copied()
    }
}

impl<'input> Extract<'input> for Document<'input> {
    fn extract(input: &'input [u8]) -> IResult<&'input [u8], Self> {
        let (input, header) = Header::parse(input)?;
        let (input, _) = take_whitespace(input)?;
        let (input, updates) = many1(Update::extract)(input)?;

        let last_update = updates
            .last()
            .expect("There should be at least one update.");

        let root = last_update.trailer.dict.root;
        let startxref = last_update.startxref;

        let mut body = Vec::new();
        let mut crossrefs = Vec::new();

        for update in updates.into_iter().rev() {
            let Update {
                body: b,
                trailer: Trailer { refs: r, .. },
                ..
            } = update;
            body.push(b);
            crossrefs.push(r)
        }

        let doc = Self {
            header,
            body: body.into_iter().flat_map(|h| h.into_iter()).collect(),
            crossrefs: crossrefs.into_iter().flatten().collect(),
            root,
            startxref,
        };

        Ok((input, doc))
    }
}
