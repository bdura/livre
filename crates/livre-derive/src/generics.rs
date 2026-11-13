//! Trait bounds added to the type parameters of a derived impl.

use std::collections::HashSet;

use syn::{GenericParam, Generics, Ident, Path, TraitBound, TraitBoundModifier, TypeParamBound};

/// Bounds every type parameter of `generics`: by `flattened_bound` when the
/// parameter is the bare type of a flattened field (listed in `flattened`),
/// by `simple` otherwise.
pub fn add_bounds(
    mut generics: Generics,
    flattened: &HashSet<Ident>,
    simple: Path,
    flattened_bound: Path,
) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(type_param) = param {
            let path = if flattened.contains(&type_param.ident) {
                flattened_bound.clone()
            } else {
                simple.clone()
            };
            type_param.bounds.push(TypeParamBound::Trait(TraitBound {
                paren_token: None,
                modifier: TraitBoundModifier::None,
                lifetimes: None,
                path,
            }));
        }
    }
    generics
}
