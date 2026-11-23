use std::collections::{HashMap, HashSet};

use winnow::{
    error::{ContextError, ErrMode},
    BStr, ModalResult,
};

use crate::{
    extraction::{extract, Extract, Indirect, Reference, ReferenceId},
    follow_refs::{Build, Builder, BuilderParser},
    structure::{Catalog, ObjectStream, Page, RefLocation, StartXRef, Trailer, XRefTrailerBlock},
};

impl Builder for HashMap<ReferenceId, &BStr> {
    fn build_reference<T>(&self, Reference { id, .. }: Reference<T>) -> ModalResult<T>
    where
        T: Build,
    {
        let input = &mut self
            .get(&id)
            .copied()
            .ok_or(ErrMode::Cut(ContextError::new()))?;

        let Indirect {
            id: reference_id,
            inner,
        } = Indirect::parse(input, self.as_parser())?;

        if reference_id != id {
            return Err(ErrMode::Cut(ContextError::new()));
        }

        Ok(inner)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct InMemoryBuilder {
    /// The entire input slice
    input: Vec<u8>,
    /// The cross-reference table
    pub xrefs: HashMap<ReferenceId, RefLocation>,
}

impl Builder for InMemoryBuilder {
    fn build_reference<T>(&self, Reference { id, .. }: Reference<T>) -> ModalResult<T>
    where
        T: Build,
    {
        let &offset = self
            .xrefs
            .get(&id)
            .ok_or(ErrMode::Backtrack(ContextError::new()))?;

        match offset {
            RefLocation::Plain(offset) => {
                let input = &mut self
                    .input
                    .get(offset..)
                    .ok_or(ErrMode::Backtrack(ContextError::new()))?
                    .as_ref();

                let Indirect {
                    id: reference_id,
                    inner,
                } = Indirect::parse(input, self.as_parser())?;

                if reference_id != id {
                    return Err(ErrMode::Cut(ContextError::new()));
                }

                Ok(inner)
            }
            RefLocation::Compressed {
                stream_id,
                // `index` is already contained within the stream.
                index: _,
            } => {
                let stream: ObjectStream =
                    self.build_reference(ReferenceId::first(stream_id).into())?;
                stream.build_object(&id, self)
            }
        }
    }
}

pub struct InMemoryDocument {
    pub catalog: Catalog,
    pub builder: InMemoryBuilder,
}

impl Extract<'_> for InMemoryDocument {
    fn extract(i: &mut &BStr) -> winnow::ModalResult<Self> {
        let input = *i;

        let StartXRef(start) = StartXRef::find(i)?;
        let i = &mut input
            .get(start..)
            .map(BStr::new)
            .ok_or(ErrMode::Cut(ContextError::new()))?;

        // Sections are visited newest-first, so the first entry seen for an object wins.
        let mut cross_references = HashMap::new();
        let mut visited = HashSet::from([start]);

        let XRefTrailerBlock {
            trailer:
                Trailer {
                    root,
                    size: _,
                    id: _,
                    mut prev,
                },
            xrefs,
        } = extract(i)?;
        for (id, location) in xrefs {
            cross_references.entry(id).or_insert(location);
        }

        // A `/Prev` pointing at an already visited section would loop forever.
        while let Some(p) = prev.filter(|&p| visited.insert(p)) {
            let i = &mut input
                .get(p..)
                .map(BStr::new)
                .ok_or(ErrMode::Cut(ContextError::new()))?;

            let XRefTrailerBlock {
                xrefs,
                trailer:
                    Trailer {
                        size: _,
                        prev: previous,
                        root: _,
                        id: _,
                    },
            } = extract(i)?;
            for (id, location) in xrefs {
                cross_references.entry(id).or_insert(location);
            }

            prev = previous;
        }

        let builder = InMemoryBuilder {
            input: input.to_vec(),
            xrefs: cross_references,
        };

        let catalog = builder.build_reference(root)?;

        Ok(Self { catalog, builder })
    }
}

impl Builder for InMemoryDocument {
    fn build_reference<T>(&self, reference: Reference<T>) -> ModalResult<T>
    where
        T: Build,
    {
        self.builder.build_reference(reference)
    }
}

impl InMemoryDocument {
    pub fn pages(&self) -> ModalResult<Vec<Page>> {
        self.catalog.pages.list_pages(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startxref_past_the_end_is_an_error() {
        let input = b"%PDF-1.7\nstartxref\n99999\n%%EOF\n";
        assert!(InMemoryDocument::extract(&mut input.as_slice().as_ref()).is_err());
    }

    #[test]
    fn prev_past_the_end_is_an_error() {
        let input = b"xref\n0 1\n0000000000 65535 f \ntrailer\n<</Size 1/Root 1 0 R/Prev 99999>>\nstartxref\n0\n%%EOF\n";
        assert!(InMemoryDocument::extract(&mut input.as_slice().as_ref()).is_err());
    }

    #[test]
    fn object_with_another_id_is_an_error() {
        let objects: HashMap<ReferenceId, &BStr> =
            [((2, 0).into(), b"3 0 obj\ntrue\nendobj".as_slice().as_ref())].into();
        let result: ModalResult<bool> = objects.build_reference(ReferenceId::from((2, 0)).into());
        assert!(result.is_err());
    }

    #[test]
    fn xref_entry_pointing_at_another_object_is_an_error() {
        let builder = InMemoryBuilder {
            input: b"3 0 obj\ntrue\nendobj".to_vec(),
            xrefs: [((2, 0).into(), RefLocation::Plain(0))].into(),
        };
        let result: ModalResult<bool> = builder.build_reference(ReferenceId::from((2, 0)).into());
        assert!(result.is_err());
    }
}
