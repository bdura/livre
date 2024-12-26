use std::fmt::Debug;

/// Location of the indirect object that a reference points to. This type is used by the
/// reference-to-byte-offset mapping.
///
/// The PDF specification allows indirect objects to be included within an object stream.
/// Livre encodes that fact within the `RefLocation` object directly.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum RefLocation {
    Plain(usize),
    Compressed(usize),
}

impl RefLocation {
    pub fn from_offset_and_flag(offset: usize, compressed: bool) -> Self {
        if compressed {
            Self::Compressed(offset)
        } else {
            Self::Plain(offset)
        }
    }
}
