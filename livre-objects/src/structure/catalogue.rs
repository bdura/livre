use livre_extraction::{FromDictRef, Reference};

#[derive(Debug, PartialEq, Eq, FromDictRef)]
pub struct Catalogue {
    // pub version: Option<Name>,
    // pub extensions
    pub pages: Reference,
}
