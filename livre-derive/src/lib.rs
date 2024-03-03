use syn::{parse_quote, GenericParam, Generics};

mod attr;
mod option;

mod extract;
mod from_dict;
mod from_dict_ref;

#[proc_macro_derive(Extract, attributes(livre))]
pub fn derive_extract(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    extract::derive(input)
}

#[proc_macro_derive(FromDict, attributes(livre))]
pub fn derive_from_dict(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    from_dict::derive(input)
}

#[proc_macro_derive(FromDictRef, attributes(livre))]
pub fn derive_from_dict_ref(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    from_dict_ref::derive(input)
}

// Add a bound `T: Extract` to every type parameter T.
fn add_trait_bounds(mut generics: Generics) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param
                .bounds
                .push(parse_quote!(livre_parser::Extract<'input>));
        }
    }
    generics
}
