//! A set of primitives that extend the [`extraction` module](crate::extraction) to follow references
//! and extract indirect objects in order to "build" more complex PDF structures.
//!
//! ## Primer on what the [`Build`] trait aims to solve
//!
//! ### Indirect objects & references
//!
//! PDF documents resort to a "random-access" architecture to be able to reuse elements and split
//! complex objects into more atomic subparts. To that end, the PDF body is an enumeration of
//! "indirect objects", which can be referenced into from other objects. PDF references thus form
//! a directed acyclic graph (DAG) since referenced object may contain reference themselves.
//!
//! Let's provide an example coming from the specification. In the following snippet, the indirect
//! object with ID `12 0` contains a string ("Brillig"):
//!
//! ```pdf
//! 12 0 obj
//! (Brillig)
//! endobj
//! ```
//!
//! This object can be referenced by another object using the reference syntax: `12 0 R`.
//!
//! Note that this is a suffix code. There is no way to know whether `12` is the number 12
//! or part of a more complex object until you reach the `R` suffix.
//!
//! ### "Building" an object with references
//!
//! In Livre, all primitive types (whether they are Rust "primitive" types or more PDF-specific,
//! e.g. the [`Name`](crate::extraction::Name) type) are [`Extract`](crate::extraction::Extract),
//! which means they declare the logic to extract themselves from a stream of bytes.
//! As a matter of fact, this includes the [`Reference`](crate::extraction::Reference) type itself,
//! as well as its [`OptRef`](crate::extraction::OptRef) sidekick. Complex objects that do not
//! rely on following and instantiating references are also simple to extract: you just need to
//! compose primitives together.
//!
//! The difficulty arises when you decide you actually want to get a complete object, rather than
//! one filled with references that are of no particular value themselves.
//!
//! This is what the `Build` trait aims to solve: a mechanism for type to be extracted from
//! a PDF document, regardless of whether some fields may be represented as references in the
//! serialisation.

mod behaviour;
mod builders;
mod parser;
mod primitive;

pub use behaviour::{Build, Builder};
pub use parser::{BuilderParser, LivreBuilder};
pub use primitive::{Built, BuiltStruct};
