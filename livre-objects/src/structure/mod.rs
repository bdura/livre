mod startxref;
pub use startxref::StartXRef;

pub mod crossref;

mod trailer;
pub use trailer::{Trailer, TrailerDict};

mod object_stream;
pub use object_stream::ObjectStream;

mod catalogue;
pub use catalogue::Catalogue;
