use std::collections::HashMap;

use livre_extraction::{extract, Extract, TypedReference};
use nom::IResult;

use livre_objects::{Indirect, Reference};

use livre_structure::{Catalogue, RefLocation, StartXRef, Trailer, TrailerDict, XRefVec};

use crate::Header;

#[derive(Debug, Clone)]
pub struct Document {
    pub header: Header,
    pub crossrefs: HashMap<Reference, RefLocation>,
    pub root: TypedReference<Catalogue>,
    pub startxref: usize,
}

impl Document {
    pub fn get_location(&self, reference: impl Into<Reference>) -> Option<RefLocation> {
        let reference = reference.into();
        println!("{reference:?}");
        self.crossrefs.get(&reference).copied()
    }
    pub fn get_referenced_bytes<'a>(
        &self,
        reference: impl Into<Reference>,
        input: &'a [u8],
    ) -> Option<&'a [u8]> {
        let location = self.get_location(reference)?;

        println!("{location:?}");

        match location {
            RefLocation::Uncompressed(loc) => {
                let (_, Indirect { inner, .. }) = extract(&input[loc..]).unwrap();
                Some(inner)
            }
            RefLocation::Compressed(_) => todo!(),
        }
    }
    pub fn parse_referenced<'input, T>(
        &self,
        reference: TypedReference<T>,
        input: &'input [u8],
    ) -> T
    where
        T: Extract<'input>,
    {
        let raw = self.get_referenced_bytes(reference, input).unwrap();
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

impl<'input> Extract<'input> for Document {
    fn extract(input: &'input [u8]) -> IResult<&'input [u8], Self> {
        let (_, header) = Header::parse(input)?;

        // Find last trailer
        let (_, StartXRef(startxref)) = StartXRef::find(&input[(input.len() - 50)..])?;
        let (_, (TrailerDict { root, .. }, crossrefs)) = find_refs(input, startxref)?;

        let doc = Self {
            // input,
            header,
            root,
            startxref,
            crossrefs: crossrefs.into_iter().rev().collect(),
        };

        Ok((input, doc))
    }
}
