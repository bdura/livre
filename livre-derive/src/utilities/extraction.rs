use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Field, Type};

use super::attr::{parse_attributes, Attributes};

pub fn extract_field(
    f: &Field,
    set: &mut HashSet<String>,
    complex: TokenStream,
    simple: TokenStream,
) -> TokenStream {
    let name = &f.ident;
    let ty = &f.ty;

    let Attributes {
        flatten,
        from,
        field_str,
        is_opt,
        default,
    } = parse_attributes(f).unwrap();

    if flatten {
        let path = match ty {
            Type::Path(p) => &p.path,
            _ => unimplemented!(),
        };
        set.insert(path.get_ident().unwrap().to_string());
    }

    let from_ty = from.as_ref().unwrap_or(ty);

    let from_ty = if default.is_some() {
        quote! {Option::<#from_ty>}
    } else {
        quote! {#from_ty}
    };

    let absent_key = if is_opt || default.is_some() {
        quote! {None}
    } else {
        quote! { return Err(::winnow::error::ErrMode::Backtrack(::winnow::error::ContextError::new())) }
    };

    let mut extraction = if flatten {
        quote! {
            let #name = #from_ty::#complex(dict, builder)?;
        }
    } else {
        quote! {
        let #name: #from_ty = if let Some(value) = dict.pop(&#field_str.into()) {
            value.#simple(builder)?
        } else {
            #absent_key
        };

        }
    };

    if let Some(default) = default {
        extraction = quote! {
            #extraction
            let #name = #name.#default;
        }
    }

    if from.is_some() {
        if is_opt {
            extraction = quote! {
                #extraction
                let #name = #name.map(|inner| inner.into());
            };
        } else {
            extraction = quote! {
                #extraction
                let #name: #ty = #name.into();
            };
        }
    }
    extraction
}
