//! A set of primitives to follow references and extract indirect object in order to "build" more
//! complex PDF objects.

mod behaviour;
mod builders;
mod parser;

pub use behaviour::{Build, Builder};
pub use parser::{BuilderParser, LivreBuilder};
