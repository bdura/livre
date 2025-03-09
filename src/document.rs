use std::collections::HashMap;

use winnow::{
    error::{ContextError, ErrMode},
    BStr, PResult,
};

use crate::{
    extraction::{extract, Extract, Indirect, Reference, ReferenceId},
    follow_refs::{Build, Builder, BuilderParser},
    structure::{Catalog, ObjectStream, Page, RefLocation, StartXRef, Trailer, XRefTrailerBlock},
};

impl Builder for HashMap<ReferenceId, &BStr> {
    fn build_reference<T>(&self, Reference { id, .. }: Reference<T>) -> PResult<T>
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

        debug_assert_eq!(reference_id, id);

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
    fn build_reference<T>(&self, Reference { id, .. }: Reference<T>) -> PResult<T>
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

                debug_assert_eq!(reference_id, id);

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
    fn extract(i: &mut &BStr) -> winnow::PResult<Self> {
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
                    mut prev,
                },
            xrefs,
        } = extract(i)?;
        cross_references.extend(xrefs);

        while let Some(p) = prev {
            let i = &mut &input[p..];

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
            cross_references.extend(xrefs);

            prev = previous;
        }

        let builder = InMemoryBuilder {
            input: input.to_vec(),
            xrefs: cross_references.into_iter().collect(),
        };

        let catalog = builder.build_reference(root)?;

        Ok(Self { catalog, builder })
    }
}

impl Builder for InMemoryDocument {
    fn build_reference<T>(&self, reference: Reference<T>) -> PResult<T>
    where
        T: Build,
    {
        self.builder.build_reference(reference)
    }
}

impl InMemoryDocument {
    pub fn pages(&self) -> PResult<Vec<Page>> {
        self.catalog.pages.list_pages(self)
    }
}
