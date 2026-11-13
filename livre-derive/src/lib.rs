//! This is not documented nor tested... I'm sorry.
//!
//! See the [`livre`](https://docs.rs/livre) documentation for more information.

use syn::{parse_macro_input, DeriveInput};

mod attrs;
mod dict;
mod extract;
mod generics;
mod plan;

/// Turns the result of a derive into tokens, errors becoming compile errors.
fn expand(result: syn::Result<proc_macro2::TokenStream>) -> proc_macro::TokenStream {
    result.unwrap_or_else(syn::Error::into_compile_error).into()
}

#[proc_macro_derive(BuildFromRawDict, attributes(livre))]
pub fn derive_build_from_raw_dict(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand(dict::derive(input, &dict::BUILD_FROM_RAW_DICT))
}

#[proc_macro_derive(FromRawDict, attributes(livre))]
pub fn derive_from_raw_dict(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand(dict::derive(input, &dict::FROM_RAW_DICT))
}

#[proc_macro_derive(Extract, attributes(livre))]
pub fn derive_extract(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand(extract::derive(parse_macro_input!(input as DeriveInput)))
}
