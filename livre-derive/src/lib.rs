//! This is not documented nor tested... I'm sorry.
//!
//! See the [`livre`](https://docs.rs/livre) documentation for more information.

use std::collections::HashSet;

use syn::{parse_quote, GenericParam, Generics};

mod build_from_raw_dict;
mod extract;
mod from_raw_dict;
mod utilities;

#[proc_macro_derive(BuildFromRawDict, attributes(livre))]
pub fn derive_build_from_raw_dict(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    build_from_raw_dict::derive(input)
}

#[proc_macro_derive(FromRawDict, attributes(livre))]
pub fn derive_from_raw_dict(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    from_raw_dict::derive(input)
}

#[proc_macro_derive(Extract, attributes(livre))]
pub fn derive_extract(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    extract::derive(input)
}

// Add a bound `T: Extract` to every type parameter T.
fn add_extraction_trait_bounds(mut generics: Generics, flattened: HashSet<String>) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            if flattened.contains(&type_param.ident.to_string()) {
                type_param
                    .bounds
                    .push(parse_quote!(crate::extraction::FromRawDict<'de>));
            } else {
                type_param
                    .bounds
                    .push(parse_quote!(crate::extraction::Extract<'de>));
            }
        }
    }
    generics
}
