use std::fmt::Debug;

/// Location of the indirect object that a reference points to. This type is used by the
/// reference-to-byte-offset mapping.
///
/// The PDF specification allows indirect objects to be included within an object stream.
/// Livre encodes that fact within the `RefLocation` object directly.
///
/// ## Side note: performance implication
///
/// We might be better off using two dedicated data structures instead of a single one with this
/// enum type, since it breaks the alignement... Do not get to attached to the `RefLocation` type,
/// it may be removed at some point.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum RefLocation {
    Plain(usize),
    Compressed { stream_id: usize, index: usize },
}
