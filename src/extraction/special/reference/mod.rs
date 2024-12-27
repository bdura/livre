mod id;
mod indirect;

#[allow(clippy::module_inception)]
mod reference;

pub use id::ReferenceId;
pub use indirect::Indirect;
pub use reference::{OptRef, Reference};
