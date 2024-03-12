use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use livre_extraction::{Extract, Reference};
use nom::{branch::alt, combinator::map};

use super::{xref_stream::XRefStream, PlainCrossRefs, RefLocation};

/// Mapping between indirect objects and the byte offset.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CrossRefs(pub HashMap<Reference, RefLocation>);

impl CrossRefs {
    /// Merge two [`CrossRefs`] objects together.
    ///
    /// Previous updates (the argument) should be overwritten.
    pub fn merge(self, older: Self) -> Self {
        // By chaining `self` _after_ `older`, we make sure that values from `self` will be kept,
        let map = older.0.into_iter().chain(self.0).collect();
        Self(map)
    }
}

impl Extract<'_> for CrossRefs {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        let (input, refs) = alt((
            map(PlainCrossRefs::extract, |PlainCrossRefs(refs)| refs),
            map(XRefStream::extract, |XRefStream { refs, .. }| refs),
        ))(input)?;
        let map = refs.into_iter().collect();
        Ok((input, Self(map)))
    }
}

impl Deref for CrossRefs {
    type Target = HashMap<Reference, RefLocation>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for CrossRefs {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
