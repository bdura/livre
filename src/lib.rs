//! Livre (pronounced [\[livʁ\]](https://en.wiktionary.org/wiki/File:Fr-un_livre-fr-ouest.ogg),
//! the French word for book) aims to provide a set of type-safe tools to read PDF content.

pub mod content;
pub mod extraction;
pub mod follow_refs;
pub mod structure;

mod document;
mod filtering;
mod utilities;

pub use document::InMemoryDocument;
