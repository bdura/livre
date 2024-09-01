use syn::{parse_quote, GenericParam, Generics};

mod extract;
mod utilities;

#[proc_macro_derive(FromRawDict, attributes(livre))]
pub fn derive_extract(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    extract::derive(input)
}

// Add a bound `T: Extract` to every type parameter T.
fn add_trait_bounds(mut generics: Generics) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param.bounds.push(parse_quote!(livre::FromDict<'de>));
        }
    }
    generics
}
