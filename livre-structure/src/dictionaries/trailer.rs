use livre_extraction::{Extract, FromDictRef, Reference};

#[derive(Debug, Clone, PartialEq, FromDictRef, Extract)]
pub struct TrailerDict {
    pub size: usize,
    pub prev: Option<usize>,
    pub root: Reference,
    // pub encrypt: Encrypt,
    pub info: Reference,
    // #[livre(rename = "ID")]
    // pub id: MaybeArray<HexString>,
}
