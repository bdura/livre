mod plain;
use plain::PlainTrailer;

mod main;
pub use main::Trailer;

mod crossref;
pub use crossref::RefLocation;
use crossref::{PlainCrossRefs, XRefStream};
