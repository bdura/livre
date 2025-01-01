//! Livre (pronounced [\[livʁ\]](https://en.wiktionary.org/wiki/File:Fr-un_livre-fr-ouest.ogg),
//! the French word for book) aims to provide a set of type-safe tools to read PDF content.

pub mod builder;
pub mod extraction;
pub mod pdf;

mod document;
mod filtering;

pub use document::InMemoryDocument;
