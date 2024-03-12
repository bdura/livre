use livre_extraction::{Extract, FromDictRef};
use livre_objects::Name;

#[derive(Debug, Clone, PartialEq, FromDictRef, Extract)]
pub struct Info {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    // pub creation_date: Option<String>,
    // pub mod_date: Option<String>,
    pub trapped: Option<Name>,
}
