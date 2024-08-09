use super::Document;

/// Build a fully qualified version of the parsed object.
/// Necessary for interacting with complex objects...
pub trait Build {
    type Output;

    fn build(self, doc: &Document) -> Self::Output;
}
