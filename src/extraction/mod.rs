//! Definition of the extraction logic, organised around one main trait:
//! [`Extract`] - and its implementation for primitive types.
//!
//! Livre aims to provide a sufficient set of tools to be able to parse *any* PDF object in a
//! type-safe way. To that end, we define an [`Extract`] trait, which defines a way for a type
//! to extract itself from a stream of bytes. Every "primitive" type (e.g. `f64`, `bool`,
//! but also some PDF-specific types) implement that trait.
//!
//! More complex objects can be extracted by composing these primitives. However, types that
//! describe more high-level aspects such as the PDF structure, the page layout or the
//! representation of fonts are not managed by this module.
//!
//! ## PDF dictionaries
//!
//! The PDF specification also defines dictionaries, that can hold heterogeneous data.
//! Livre can represent those as generic [`HashMap`](std::collections::HashMap)s,
//! or extract them as structured types using the [`FromRawDict`] trait.
//!
//! Using this trait instead of [`Extract`] directly lets us use more complex extraction patterns.
//!
//! ## Derivability
//!
//! Through the [`livre_derive`] helper crate, Livre provides (and uses) a derive macro
//! for [`FromRawDict`], making the derived type indirectly [`Extract`] through a blanket
//! implementation.
//!
//! ## Indirect objects
//!
//! The PDF specification allows references to *indirect objects* (see section 7.3.10 from
//! the specification). Thus, Livre needs to defines a way for the parsing logic to follow
//! these references in order to allow the reconstruction of any PDF object.
//!
//! Livre defines two special types to represent references:
//!
//! - [`Reference`] is a generic type that holds a reference to an indirect object.
//! - [`OptRef`], also generic, contains either the object itself, or a reference to it.
//!
//! These two types are [`Extract`]. However, this module does not define any mechanism
//! to follow the reference and extract an indirect object. See the
//! [`follow_refs` module](crate::follow_refs) for that.

mod extract;
mod from_raw_dict;
mod primitives;
mod special;
mod utilities;

pub use extract::{extract, Extract};
pub use from_raw_dict::FromRawDict;

pub use special::{
    multicomment0, multicomment1, Comment, Date, HexadecimalString, Id, Indirect, LiteralString,
    Map, MaybeArray, Name, Nil, Object, OptRef, PDFString, RawDict, Rectangle, Reference,
    ReferenceId, Stream, Todo,
};

pub(crate) use utilities::{take_till_delimiter, Angles, Brackets, Parentheses};
