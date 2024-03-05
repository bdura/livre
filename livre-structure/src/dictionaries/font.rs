use livre_extraction::{Extract, FromDictRef};
use livre_objects::Name;

#[derive(Extract, FromDictRef)]
pub struct FontDict {
    pub sub_type: Name,
}
