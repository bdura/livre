pub mod error;

pub mod primitives;

mod utilities;
pub use utilities::{MaybeArray, OptRef, RawDict};

pub mod pdf;
pub use pdf::{HexString, Indirect, Name, Reference};

pub mod extraction;
pub use extraction::{Extract, FromDict, Parse};

pub use livre_derive::{Extract, FromDict};

/// Re-export IResult. Avoids depending on `nom` for downstream crates.
pub use nom::IResult;
