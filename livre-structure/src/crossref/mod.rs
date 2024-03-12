mod location;
pub use location::{Ref, RefLocation};

mod main;
pub use main::CrossRefs;

mod plain;
use plain::PlainCrossRefs;

mod xref_stream;
