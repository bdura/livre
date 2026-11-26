//! Definition of object streams.

use std::collections::HashMap;
use std::iter::Iterator;

use winnow::{
    combinator::{iterator, preceded, trace},
    error::{ContextError, ErrMode},
    BStr, ModalResult, Parser,
};

use crate::{
    extraction::{extract, multicomment0, Reference, ReferenceId, Stream},
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
/// We are making the assumption that a cross-reference stream always points to the
/// `ObjectStream` that holds the object, so an object is only looked up in the stream it is
/// asked from. The reference is kept ([`extends`](Self::extends)) but the extended stream is
/// never read.
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
        let &offset = self.map.get(reference)?;
        self.content.get(offset..).map(AsRef::as_ref)
    }
}

impl ObjectStream {
    /// Build an object contained within the `ObjectStream`. Returns an error if the key is absent.
    pub fn build_object<B, T>(&self, reference: &ReferenceId, builder: &B) -> ModalResult<T>
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
    /// `/Extends`, the object stream this one extends, if any. It is not read: see the notes
    /// on [`ObjectStream`].
    pub fn extends(&self) -> Option<Reference<ObjectStream>> {
        self.extends
    }
}

impl Build for ObjectStream {
    fn build<B>(input: &mut &BStr, builder: &B) -> ModalResult<Self>
    where
        B: Builder,
    {
        trace("livre-object-stream", move |i: &mut &BStr| {
            let Stream {
                structured: ObjectStreamDict { n, first, extends },
                content,
            } = builder.as_parser().parse_next(i)?;

            let (i, content) = content
                .split_at_checked(first)
                .ok_or(ErrMode::Cut(ContextError::new()))?;
            let content = content.to_vec();

            let mut stream_dict = BStr::new(&i);
            let mut it = iterator(&mut stream_dict, preceded(multicomment0, extract));

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
    fn first_past_the_content_is_an_error() {
        let input =
            b"<</Type/ObjStm/Length 18/N 2/First 100>>\nstream\n11 0 12 50\ntrue 42\nendstream";
        let result: ModalResult<ObjectStream> =
            ().as_parser().parse_next(&mut input.as_slice().as_ref());
        assert!(result.is_err());
    }

    #[test]
    fn object_offset_past_the_content_is_an_error() {
        let input =
            b"<</Type/ObjStm/Length 18/N 2/First 11>>\nstream\n11 0 12 50\ntrue 42\nendstream";
        let stream: ObjectStream =
            ().as_parser()
                .parse_next(&mut input.as_slice().as_ref())
                .unwrap();
        let result: ModalResult<bool> = stream.build_object(&(12, 0).into(), &());
        assert!(result.is_err());
    }

    /// `/Extends` names another object stream: it is kept as a reference, and that stream is
    /// not read, even through a builder that knows it.
    #[test]
    fn extends_is_kept_as_a_reference() {
        let extended: &[u8] =
            b"9 0 obj\n<</Type/ObjStm/Length 8/N 1/First 5>>\nstream\n12 0\n43\nendstream\nendobj";
        let builder: HashMap<ReferenceId, &BStr> =
            [((9, 0).into(), extended.into())].into_iter().collect();

        let input: &[u8] =
            b"<</Type/ObjStm/Length 8/N 1/First 5/Extends 9 0 R>>\nstream\n11 0\n42\nendstream";
        let stream: ObjectStream = builder.as_parser().parse_next(&mut input.into()).unwrap();

        assert_eq!(stream.extends(), Some((9, 0).into()));
        assert_eq!(
            stream.build_object::<_, u32>(&(11, 0).into(), &builder),
            Ok(42)
        );
        assert!(stream
            .build_object::<_, u32>(&(12, 0).into(), &builder)
            .is_err());
    }

    #[test]
    fn object_stream() {
        let input = indoc! {b"
            <<
                /Type /ObjStm
                /Length 38
                /N 3
                /First 23
            >>
            stream
            11 0 % first
            12 5 13 8
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
