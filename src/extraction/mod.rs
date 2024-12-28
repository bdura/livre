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
//! ## Derivability
//!
//! Through the [`livre_derive`] helper crate, Livre provides (and uses) a derive macro
//! for [`FromRawDict`], making the derived type [`Extract`].
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
//! [`builder` module](crate::builder) for that.

mod behaviour;
mod primitives;
mod special;
mod utilities;

pub use behaviour::{extract, Extract, FromRawDict};

pub use special::{
    multicomment0, multicomment1, Comment, HexadecimalString, Indirect, LiteralString, MaybeArray,
    Name, Nil, Object, OptRef, RawDict, Reference, ReferenceId, Stream,
};
