use std::collections::HashMap;

use crate::parsers::{extract, Extract, TypedReference};
use nom::IResult;

use crate::objects::{Indirect, Reference};

use crate::structure::{Catalogue, RefLocation, StartXRef, Trailer, TrailerDict, XRefVec};

use super::Header;

#[derive(Debug, Clone)]
pub struct DocumentBuilder<'input> {
    pub input: &'input [u8],
    pub header: Header,
    pub crossrefs: HashMap<Reference, RefLocation>,
    pub root: TypedReference<Catalogue>,
    pub startxref: usize,
}

impl<'input> DocumentBuilder<'input> {
    pub fn get_location(&self, reference: impl Into<Reference>) -> Option<RefLocation> {
        let reference = reference.into();
        self.crossrefs.get(&reference).copied()
    }

    pub fn get_referenced_bytes(&self, reference: impl Into<Reference>) -> Option<&[u8]> {
        let location = self.get_location(reference)?;

        match location {
            RefLocation::Uncompressed(loc) => {
                let (_, Indirect { inner, .. }) = extract(&self.input[loc..]).unwrap();
                Some(inner)
            }
            RefLocation::Compressed(_) => todo!(),
        }
    }

    pub fn parse_referenced<T>(&'input self, reference: impl Into<TypedReference<T>>) -> T
    where
        T: Extract<'input>,
    {
        let raw = self.get_referenced_bytes(reference.into()).unwrap();
        let (_, inner) = extract(raw).unwrap();
        inner
    }
}

fn find_refs(input: &[u8], prev: usize) -> IResult<&[u8], (TrailerDict, XRefVec)> {
    let (_, Trailer { dict, mut refs }) = extract(&input[prev..])?;

    if let Some(prev) = dict.prev {
        let (_, (_, prev_refs)) = find_refs(input, prev)?;
        refs.extend(prev_refs)
    }

    Ok((input, (dict, refs)))
}

impl<'input> Extract<'input> for DocumentBuilder<'input> {
    fn extract(input: &'input [u8]) -> IResult<&'input [u8], Self> {
        let (_, header) = Header::extract(input)?;

        // Find last trailer
        let (_, StartXRef(startxref)) = StartXRef::find(&input[(input.len() - 50)..])?;
        let (_, (TrailerDict { root, .. }, crossrefs)) = find_refs(input, startxref)?;

        let doc = Self {
            input,
            header,
            root,
            startxref,
            crossrefs: crossrefs.into_iter().rev().collect(),
        };

        Ok((input, doc))
    }
}
