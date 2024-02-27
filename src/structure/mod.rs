//! Structure of the PDF (section 7.5 in the specs.)

mod crossrefs;
pub use crossrefs::CrossRefs;

mod header;
pub use header::{Header, Version};

mod trailer;
pub use trailer::Trailer;

mod startxref;
pub use startxref::StartXRef;

mod update;
pub use update::Update;

mod document;
pub use document::Document;
