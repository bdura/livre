use serde::Deserialize;

use crate::{
    objects::{Object, Reference},
    parsers::OptRef,
};

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Type0 {
    pub descendant_fonts: OptRef<Vec<OptRef<Object>>>,
    pub encoding: String,
    pub to_unicode: Option<Reference>,
}
