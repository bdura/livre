/// Re-export usefull extractors from [`livre-extraction`].
pub use livre_extraction::{HexBytes, Indirect, Name, Reference};

mod stream;
pub use stream::{Bytes, Stream};

mod object;
pub use object::Object;
