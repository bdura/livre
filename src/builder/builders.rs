use winnow::BStr;

use crate::extraction::ReferenceId;

use super::Builder;

/// The unit type is a context-less builder, making `().as_parser` somewhat equivalent to
/// `extact`: it will simply error if there is any reference to instantiate.
impl<'de> Builder<'de> for () {
    fn follow_reference(&self, _reference_id: ReferenceId) -> Option<&'de BStr> {
        None
    }
}
