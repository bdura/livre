//! Definition of object streams.

use std::collections::HashMap;
use std::iter::Iterator;

use winnow::{
    ascii::multispace0,
    combinator::{iterator, preceded, trace},
    error::{ContextError, ErrMode},
    BStr, PResult, Parser,
};

use crate::{
    debug,
    extraction::{extract, Reference, ReferenceId, Stream},
    follow_refs::{Build, BuildFromRawDict, Builder, BuilderParser},
};

/// Represents a PDF object stream.
///
/// Object streams were introduced in version 1.5, and provide a way to store indirect objects more
/// efficiently through the use of streams, which can be compressed.
///
/// In Livre, the `ObjectStream` type only keeps track of the actual data, while the stream
/// structure itself is considered an implementation detail.
///
/// ## Notes
///
/// The PDF specification uses an `Extends` key to generate a linked collection of compressed
/// objects. However, it is unclear whether a compressed object _could_ be referenced indirectly,
/// that is, through a separate `ObjectStream` that extends the one that contains it.
///
/// For now, we are making the assumption that a reference stream will always point to the correct
/// `ObjectStream`, and do not instantiate the full object stream. However, in case this assumption
/// is wrong, we keep the PDF reference around to instantiate it if need be. Note that this option
/// is only available to owned types (see [`build_owned_object`](Self::build_owned_object)) for
/// more detail.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ObjectStream {
    /// Mapping from ReferenceId to byte offset within the content
    map: HashMap<ReferenceId, usize>,
    /// Content decoded from the stream, stripped of the header
    content: Vec<u8>,
    /// Optional reference to a previous ObjectStream.
    extends: Option<Reference<ObjectStream>>,
}

impl ObjectStream {
    /// Get the byte stream associated with a reference.
    fn get_data(&self, reference: &ReferenceId) -> Option<&BStr> {
        self.map
            .get(reference)
            .map(|&offset| self.content[offset..].as_ref())
    }
}

impl ObjectStream {
    /// Build an object contained within the `ObjectStream`. Returns an error if the key is absent.
    pub fn build_object<B, T>(&self, reference: &ReferenceId, builder: &B) -> PResult<T>
    where
        T: Build,
        B: Builder,
    {
        let mut input = self
            .get_data(reference)
            .ok_or(ErrMode::Cut(ContextError::new()))?;

        builder.as_parser().parse_next(&mut input)
    }
}

impl ObjectStream {
    /// Owned types can be built following the linked list of `ObjectStream` if need be.
    ///
    /// This is only available to owned types because while following the linked list, we
    /// instantiate transient objects that cannot be referenced into.
    pub fn build_owned_object<B, T>(&self, reference: &ReferenceId, builder: &B) -> PResult<T>
    where
        T: Build,
        B: Builder,
    {
        if let Some(mut input) = self.get_data(reference) {
            builder.as_parser().parse_next(&mut input)
        } else if let Some(extends) = self.extends {
            // FIXME: use some actual logging framework.

            debug!("Reference not found in this stream. Checking the extended stream.");

            let stream = builder.build_reference(extends)?;
            stream.build_object(reference, builder)
        } else {
            Err(ErrMode::Backtrack(ContextError::new()))
        }
    }
}

impl Build for ObjectStream {
    fn build<B>(input: &mut &BStr, builder: &B) -> PResult<Self>
    where
        B: Builder,
    {
        trace("livre-object-stream", move |i: &mut &BStr| {
            let Stream {
                structured: ObjectStreamDict { n, first, extends },
                content,
            } = builder.as_parser().parse_next(i)?;

            let (i, content) = content.split_at(first);
            let content = content.to_vec();

            let mut it = iterator(i.as_ref(), preceded(multispace0, extract));

            let map = it
                .map(|(object, offset)| (ReferenceId::first(object), offset))
                .take(n)
                .collect();
            it.finish()?;

            Ok(Self {
                map,
                content,
                extends,
            })
        })
        .parse_next(input)
    }
}

/// Transient object that represents the structured data associated with the [`Stream`], containing
/// necessary information for the extraction of the [`ObjectStream`].
#[derive(BuildFromRawDict)]
struct ObjectStreamDict {
    /// Number of elements within the object stream
    pub n: usize,
    /// Byte offset of the **first** serialised element in the object stream decoded content.
    pub first: usize,
    /// An optional reference to another object stream.
    pub extends: Option<Reference<ObjectStream>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    use indoc::indoc;

    #[test]
    fn object_stream() {
        let input = indoc! {b"
            <<
                /Type /ObjStm
                /Length 30
                /N 3
                /First 15
            >>
            stream
            11 0 12 5 13 8
            true
            42
            (test)
            endstream
        "}
        .as_slice();

        let stream: ObjectStream = ().as_parser().parse_next(&mut input.as_ref()).unwrap();
        let expected = ObjectStream {
            map: vec![((11usize, 0u16), 0usize), ((12, 0), 5), ((13, 0), 8)]
                .into_iter()
                .map(|(a, b)| (a.into(), b))
                .collect(),
            content: indoc! {b"
                true
                42
                (test)
            "}
            .into(),
            extends: None,
        };

        assert_eq!(stream, expected);
    }
}
