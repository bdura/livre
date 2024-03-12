use livre_extraction::{Extract, FromDictRef, TypedReference};

use crate::Catalogue;

use super::Info;

#[derive(Debug, Clone, PartialEq, FromDictRef, Extract)]
pub struct TrailerDict {
    pub size: usize,
    pub prev: Option<usize>,
    pub root: TypedReference<Catalogue>,
    // pub encrypt: Encrypt,
    pub info: TypedReference<Info>,
    // #[livre(rename = "ID")]
    // pub id: MaybeArray<HexString>,
}
