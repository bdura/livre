mod location;
pub use location::{Ref, RefLocation};

mod main;
pub use main::CrossRefs;

mod plain;
pub use plain::PlainCrossRefs;

mod xref_stream;
pub use xref_stream::XRefStream;
