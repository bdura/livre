use winnow::BStr;

use crate::{builder::Builder, extraction::ReferenceId};

/// The dummy builder, which does not follow references at all.
impl<'de> Builder<'de> for () {
    fn follow_reference(&self, _reference_id: ReferenceId) -> Option<&'de BStr> {
        None
    }
}
