use winnow::{BStr, ModalResult};

use crate::{extraction::Extract, follow_refs::Build};

/// Placeholder element, does **not** consume any input, and is therefore only suited for
/// dictionary values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Todo;

impl Extract<'_> for Todo {
    fn extract(_: &mut &BStr) -> ModalResult<Self> {
        Ok(Self)
    }
}

impl Build for Todo {
    fn build<B>(_: &mut &BStr, _: &B) -> ModalResult<Self>
    where
        B: crate::follow_refs::Builder,
    {
        Ok(Self)
    }
}
