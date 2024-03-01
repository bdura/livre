/// Re-export usefull extractors from [`livre-extraction`].
pub use livre_extraction::{HexString, Indirect, Name, Reference};

mod objects;
pub use objects::{Object, Stream};
