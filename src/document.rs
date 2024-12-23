use std::collections::HashMap;

use winnow::BStr;

use crate::{
    extraction::{Builder, Extract, Reference, ReferenceId},
    pdf::{extract_xref, Catalog, RefLocation, StartXRef, TrailerDict},
};

impl<'de> Builder<'de> for HashMap<ReferenceId, &'de BStr> {
    fn follow_reference(&self, reference_id: ReferenceId) -> Option<&'de BStr> {
        self.get(&reference_id).copied()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct InMemoryDocument<'de> {
    pub catalog: Reference<Catalog>,
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
            RefLocation::Uncompressed(offset) => self.input.get(offset..).map(|s| s.as_ref()),
            _ => todo!("We focus on *vanilla* refs for now."),
        }
    }
}

impl<'de> Extract<'de> for InMemoryDocument<'de> {
    fn extract(i: &mut &'de BStr) -> winnow::PResult<Self> {
        let input = *i;

        let StartXRef(start) = StartXRef::find(i)?;
        let i = &mut &input[start..];

        let mut xrefs = Vec::new();

        let (TrailerDict { prev, root, .. }, refs) = extract_xref(i)?;
        xrefs.extend(refs);

        let mut next = prev;

        while let Some(prev) = next {
            let i = &mut &input[prev..];

            let (TrailerDict { prev, .. }, refs) = extract_xref(i)?;
            xrefs.extend(refs);

            next = prev;
        }

        Ok(Self {
            catalog: root,
            input,
            xrefs: xrefs.into_iter().collect(),
        })
    }
}
