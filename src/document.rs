use std::collections::HashMap;

use winnow::BStr;

use crate::{
    extraction::{extract, Builder, Extract, Reference, ReferenceId},
    pdf::{Previous, RefLocation, StartXRef, Trailer, XRefTrailerBlock},
};

impl<'de> Builder<'de> for HashMap<ReferenceId, &'de BStr> {
    fn follow_reference(&self, reference_id: ReferenceId) -> Option<&'de BStr> {
        self.get(&reference_id).copied()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct InMemoryDocument<'de> {
    pub catalog: Reference<()>,
    /// The entire input slice
    input: &'de BStr,
    /// The cross-reference table
    /// TODO: add stream support: some references can be put behind a stream.
    pub xrefs: HashMap<ReferenceId, RefLocation>,
}

impl<'de> Builder<'de> for InMemoryDocument<'de> {
    fn follow_reference(&self, reference_id: ReferenceId) -> Option<&'de BStr> {
        let &offset = self.xrefs.get(&reference_id)?;

        match offset {
            RefLocation::Plain(offset) => self.input.get(offset..).map(|s| s.as_ref()),
            _ => todo!("We focus on *vanilla* refs for now."),
        }
    }
}

impl<'de> Extract<'de> for InMemoryDocument<'de> {
    fn extract(i: &mut &'de BStr) -> winnow::PResult<Self> {
        let input = *i;

        let StartXRef(start) = StartXRef::find(i)?;
        let i = &mut &input[start..];

        let mut cross_references = Vec::new();

        let XRefTrailerBlock {
            trailer:
                Trailer {
                    root,
                    size: _,
                    id: _,
                },
            mut previous,
            xrefs,
        } = extract(i)?;
        cross_references.extend(xrefs);

        while let Previous::Linked(prev) = previous {
            let i = &mut &input[prev..];

            let XRefTrailerBlock {
                previous: p,
                xrefs,
                trailer: _,
            } = extract(i)?;
            cross_references.extend(xrefs);

            previous = p;
        }

        Ok(Self {
            catalog: root,
            input,
            xrefs: cross_references.into_iter().collect(),
        })
    }
}
