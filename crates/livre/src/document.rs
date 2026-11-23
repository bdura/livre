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

        // Sections are visited newest-first, so the first entry seen for an object wins.
        let mut cross_references = HashMap::new();
        let mut root = None;
        let mut visited = HashSet::new();
        let mut next = Some(start);

        // A `/Prev` pointing at an already visited section would loop forever.
        while let Some(offset) = next.filter(|&o| visited.insert(o)) {
            let i = &mut input
                .get(offset..)
                .map(BStr::new)
                .ok_or(ErrMode::Cut(ContextError::new()))?;

            // Older trailers are parsed in full, `/Root` included, though only the newest is used.
            let XRefTrailerBlock {
                xrefs,
                trailer:
                    Trailer {
                        size: _,
                        prev,
                        root: section_root,
                        id: _,
                    },
            } = extract(i)?;
            for (id, location) in xrefs {
                cross_references.entry(id).or_insert(location);
            }
            root.get_or_insert(section_root);

            next = prev;
        }

        // The first turn always runs, so `root` is set; erroring keeps the crate panic-free.
        let root = root.ok_or(ErrMode::Cut(ContextError::new()))?;

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
    fn prev_cycle_terminates() {
        let input = b"xref\n0 1\n0000000000 65535 f \ntrailer\n<</Size 1/Root 1 0 R/Prev 0>>\nstartxref\n0\n%%EOF\n";
        // Only termination matters here: `Ok` and `Err` are both acceptable.
        let _ = InMemoryDocument::extract(&mut input.as_slice().as_ref());
    }

    #[test]
    fn newest_section_wins() {
        let catalog = "1 0 obj\n<</Type/Catalog/Pages 2 0 R>>\nendobj\n";
        let pages = "2 0 obj\n<</Type/Pages/Kids[]/Count 0>>\nendobj\n";

        let mut input = String::from("%PDF-1.7\n");
        let old_catalog = input.len();
        input += catalog;
        let pages_offset = input.len();
        input += pages;
        let old_xref = input.len();
        input += &format!(
            "xref\n0 3\n0000000000 65535 f \n{old_catalog:010} 00000 n \n{pages_offset:010} 00000 n \n\
             trailer\n<</Size 3/Root 1 0 R>>\n"
        );
        let new_catalog = input.len();
        input += catalog;
        let new_xref = input.len();
        input += &format!(
            "xref\n0 2\n0000000000 65535 f \n{new_catalog:010} 00000 n \n\
             trailer\n<</Size 3/Root 1 0 R/Prev {old_xref}>>\nstartxref\n{new_xref}\n%%EOF\n"
        );

        let document = InMemoryDocument::extract(&mut input.as_bytes().as_ref()).unwrap();
        assert_eq!(
            document.builder.xrefs[&ReferenceId::from((1, 0))],
            RefLocation::Plain(new_catalog)
        );
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
