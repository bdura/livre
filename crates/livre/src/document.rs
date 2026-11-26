use std::{
    collections::{HashMap, HashSet},
    fmt,
    sync::{Arc, Mutex, MutexGuard},
};

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

/// Decoded object streams, keyed by the object number of the stream.
///
/// All the objects of an object stream share one compressed stream: without the cache, each
/// of them would inflate the stream and parse its header again.
///
/// The cache is derived from the input, so it is not part of a builder's value: `Clone`
/// returns an empty cache and `PartialEq` is always true, which lets [`InMemoryBuilder`] keep
/// its derives and two builders over the same bytes compare equal.
#[derive(Default)]
struct ObjectStreamCache(Mutex<HashMap<usize, Arc<ObjectStream>>>);

impl ObjectStreamCache {
    /// Locks the map. A poisoned lock is recovered: the map only ever holds finished streams,
    /// so a panic elsewhere cannot leave it half-written.
    fn lock(&self) -> MutexGuard<'_, HashMap<usize, Arc<ObjectStream>>> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn get(&self, stream_id: usize) -> Option<Arc<ObjectStream>> {
        self.lock().get(&stream_id).cloned()
    }

    /// Stores `stream` unless another thread stored one first; returns the stream kept.
    fn insert(&self, stream_id: usize, stream: Arc<ObjectStream>) -> Arc<ObjectStream> {
        self.lock().entry(stream_id).or_insert(stream).clone()
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.lock().len()
    }
}

impl Clone for ObjectStreamCache {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl PartialEq for ObjectStreamCache {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl fmt::Debug for ObjectStreamCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ObjectStreamCache")
            .field("streams", &self.lock().len())
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct InMemoryBuilder {
    /// The entire input slice
    input: Vec<u8>,
    /// The cross-reference table
    pub xrefs: HashMap<ReferenceId, RefLocation>,
    /// Object streams decoded so far
    object_streams: ObjectStreamCache,
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
                let stream = match self.object_streams.get(stream_id) {
                    Some(stream) => stream,
                    None => {
                        // No lock is held while building: the stream's dictionary may hold
                        // references, which re-enter this method.
                        let stream: ObjectStream =
                            self.build_reference(ReferenceId::first(stream_id).into())?;
                        self.object_streams.insert(stream_id, Arc::new(stream))
                    }
                };
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
            object_streams: ObjectStreamCache::default(),
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

    use crate::extraction::Object;

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
            object_streams: ObjectStreamCache::default(),
        };
        let result: ModalResult<bool> = builder.build_reference(ReferenceId::from((2, 0)).into());
        assert!(result.is_err());
    }

    fn read_resource(path: &str) -> InMemoryDocument {
        let bytes = std::fs::read(path).unwrap();
        InMemoryDocument::extract(&mut bytes.as_slice().as_ref()).unwrap()
    }

    /// Two objects stored in the same object stream, with that stream's object number.
    fn two_compressed_objects(builder: &InMemoryBuilder) -> (usize, ReferenceId, ReferenceId) {
        let mut by_stream: HashMap<usize, Vec<ReferenceId>> = HashMap::new();
        for (&id, &location) in &builder.xrefs {
            if let RefLocation::Compressed { stream_id, .. } = location {
                by_stream.entry(stream_id).or_default().push(id);
            }
        }
        let (stream_id, ids) = by_stream
            .into_iter()
            .find(|(_, ids)| ids.len() >= 2)
            .expect("an object stream holding two objects");
        (stream_id, ids[0], ids[1])
    }

    #[test]
    fn compressed_objects_share_one_decoded_stream() {
        let document = read_resource("tests/resources/text-xref-stream.pdf");
        // Extracting the document already warmed its builder's cache: start from an empty one.
        let builder = InMemoryBuilder {
            object_streams: ObjectStreamCache::default(),
            ..document.builder
        };
        let (stream_id, first, second) = two_compressed_objects(&builder);

        let _: Object = builder.build_reference(first.into()).unwrap();
        let _: Object = builder.build_reference(second.into()).unwrap();

        assert_eq!(builder.object_streams.len(), 1);
        assert!(builder.object_streams.get(stream_id).is_some());
    }

    #[test]
    fn failed_object_stream_is_not_cached() {
        // Object 5 is a boolean, not an object stream.
        let builder = InMemoryBuilder {
            input: b"5 0 obj\ntrue\nendobj".to_vec(),
            xrefs: [
                (
                    (1, 0).into(),
                    RefLocation::Compressed {
                        stream_id: 5,
                        index: 0,
                    },
                ),
                ((5, 0).into(), RefLocation::Plain(0)),
            ]
            .into(),
            object_streams: ObjectStreamCache::default(),
        };

        for _ in 0..2 {
            let result: ModalResult<bool> =
                builder.build_reference(ReferenceId::from((1, 0)).into());
            assert!(result.is_err());
        }
        assert_eq!(builder.object_streams.len(), 0);
    }

    #[test]
    fn cloned_builder_starts_with_an_empty_cache_and_compares_equal() {
        let document = read_resource("tests/resources/text-xref-stream.pdf");
        let builder = document.builder;
        let (_, id, _) = two_compressed_objects(&builder);
        let expected: Object = builder.build_reference(id.into()).unwrap();
        assert!(builder.object_streams.len() > 0);

        let clone = builder.clone();

        assert_eq!(clone.object_streams.len(), 0);
        assert_eq!(clone, builder);
        let actual: Object = clone.build_reference(id.into()).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn in_memory_builder_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<InMemoryBuilder>();
        assert_send_sync::<InMemoryDocument>();
    }
}
