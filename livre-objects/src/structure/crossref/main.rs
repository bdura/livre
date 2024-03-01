use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use livre_extraction::Reference;

/// Mapping between indirect objects and the byte offset.
#[derive(Debug, Clone, Default)]
pub struct CrossRefs(pub HashMap<Reference, usize>);

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

impl Deref for CrossRefs {
    type Target = HashMap<Reference, usize>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for CrossRefs {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
