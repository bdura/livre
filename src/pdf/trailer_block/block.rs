use std::fmt::Debug;

use winnow::{
    combinator::{alt, trace},
    BStr, PResult, Parser,
};

use crate::{
    extraction::{Extract, ReferenceId},
    pdf::Trailer,
};

use super::{plain, stream, RefLocation};

/// Container for a cross-reference table & PDF trailer block.
///
/// Even though the PDF specification draws a distinction between the cross-reference table and the
/// PDF trailer, the two are strongly correlated. Starting in PDF 1.5, the cross-reference table can
/// be expressed using a cross-reference **stream**, which uses a [`Stream`](crate::extraction::Stream)
/// object whose [structured data](crate::extraction::Stream::structured) **is** the PDF trailer,
/// making extracting one without the other pointless.
///
/// To cope with this string correlation, and allow extraction of two competing representations for
/// the cross-reference table, Livre defines the `XRefTrailerBlock` abstraction that handles both
/// the xref table and the trailer together.
///
/// ## Side-node: some performance considerations
///
/// Only the very last trailer is actually needed to understand the PDF (unless the edit history is
/// important). I have looked into making this type generic over the `trailer` field, which would
/// allow extracting the [`Nil` object](crate::extraction::Nil) during the iteration over previous
/// blocks, and thus avoid instantiating a full [`Trailer`] object every time.
///
/// However, in the case of the cross-reference **stream** configuration, the `Size` key may be
/// needed to extract cross-references. Some ideas to cope with that:
///
/// - remove the key from the `Trailer` dictionary, but that would stray us further from the PDF
///   specification.
/// - collect the `Size` key without poping it from the `RawDict`, but we would collect the same
///   object twice... I think the moral is: let's benchmark that at a later time.
#[derive(Debug, PartialEq)]
pub struct XRefTrailerBlock {
    /// The [PDF trailer](Trailer). Only the last one (i.e., the first one that is extracted) is
    /// useful.
    pub trailer: Trailer,
    /// The collected cross-reference. The full cross-reference table is the concatenation of all
    /// tables.
    pub xrefs: Vec<(ReferenceId, RefLocation)>,
}

impl Extract<'_> for XRefTrailerBlock {
    fn extract(input: &mut &'_ BStr) -> PResult<Self> {
        trace("livre-xref-block", alt((plain::block, stream::block))).parse_next(input)
    }
}
