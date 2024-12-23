//! Livre (pronounced [[livʁ]](https://en.wiktionary.org/wiki/File:Fr-un_livre-fr-ouest.ogg),
//! the French word for book) aims to provide a set of type-safe tools to read PDF content.

pub mod extraction;

mod document;
mod filtering;
mod pdf;
mod structures;

pub use document::InMemoryDocument;
pub use pdf::Page;
pub use structures::Rectangle;
