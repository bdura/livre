use livre_extraction::{Extract, FromDictRef, Reference, TypedReference};

use crate::Catalogue;

#[derive(Debug, Clone, PartialEq, FromDictRef, Extract)]
pub struct TrailerDict {
    pub size: usize,
    pub prev: Option<usize>,
    pub root: TypedReference<Catalogue>,
    // pub encrypt: Encrypt,
    pub info: Reference,
    // #[livre(rename = "ID")]
    // pub id: MaybeArray<HexString>,
}
