//! This is not documented nor tested... I'm sorry.
//!
//! See the [`livre`](https://docs.rs/livre) documentation for more information.

use std::collections::HashSet;

use syn::{parse_macro_input, parse_quote, DeriveInput, GenericParam, Generics};

mod attrs;
mod build_from_raw_dict;
mod dict;
mod extract;
mod from_raw_dict;
mod plan;

/// Runs `derive` on the parsed input, turning its error into a compile error.
fn expand(
    input: DeriveInput,
    derive: fn(DeriveInput) -> syn::Result<proc_macro2::TokenStream>,
) -> proc_macro::TokenStream {
    derive(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(BuildFromRawDict, attributes(livre))]
pub fn derive_build_from_raw_dict(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand(
        parse_macro_input!(input as DeriveInput),
        build_from_raw_dict::derive,
    )
}

#[proc_macro_derive(FromRawDict, attributes(livre))]
pub fn derive_from_raw_dict(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand(
        parse_macro_input!(input as DeriveInput),
        from_raw_dict::derive,
    )
}

#[proc_macro_derive(Extract, attributes(livre))]
pub fn derive_extract(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand(parse_macro_input!(input as DeriveInput), extract::derive)
}

// Add a bound `T: Extract` to every type parameter T.
fn add_extraction_trait_bounds(mut generics: Generics, flattened: HashSet<String>) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            if flattened.contains(&type_param.ident.to_string()) {
                type_param
                    .bounds
                    .push(parse_quote!(::livre::extraction::FromRawDict<'de>));
            } else {
                type_param
                    .bounds
                    .push(parse_quote!(::livre::extraction::Extract<'de>));
            }
        }
    }
    generics
}
