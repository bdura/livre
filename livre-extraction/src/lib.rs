pub mod error;

pub mod primitives;
pub use primitives::map::Map;

mod utilities;
pub use utilities::{MaybeArray, OptRef, RawDict};

pub mod pdf;
pub use pdf::{HexString, Indirect, Name, Reference};

pub mod extraction;
pub use extraction::{Extract, FromDict, FromDictRef, Parse};

pub use livre_derive::{Extract, FromDict, FromDictRef};

pub mod implementations;

/// Re-export IResult. Avoids depending on `nom` for downstream crates.
pub use nom::IResult;
