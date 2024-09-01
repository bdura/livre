use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields};

use crate::{add_trait_bounds, utilities::attr::Attributes};

use super::utilities::attr;

pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parse the input tokens into a syntax tree.
    let input = parse_macro_input!(input as DeriveInput);

    // Used in the quasi-quotation below as `#name`.
    let name = input.ident;

    // Add a bound `T: Extract` to every type parameter T.
    let generics = add_trait_bounds(input.generics);
    let (_impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let extraction = generate_extraction(&input.data);

    let expanded = quote! {
        // The generated impl.
        impl<'de> crate::extraction::FromRawDict<'de> for #name #ty_generics #where_clause {
            fn from_raw_dict(dict: &mut crate::extraction::RawDict<'de>) -> ::winnow::PResult<Self> {
                #extraction
                Ok(res)
            }
        }
    };

    // Hand the output tokens back to the compiler.
    proc_macro::TokenStream::from(expanded)
}

fn generate_extraction(data: &Data) -> TokenStream {
    match data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => {
            let fieldname = fields.named.iter().map(|f| &f.ident).collect::<Vec<_>>();

            let field_by_field = fields.named.iter().map(|f| {
                let name = &f.ident;
                let ty = &f.ty;

                let Attributes {
                    flatten,
                    from,
                    field_str,
                    is_opt,
                    default,
                } = attr::parse_attributes(f).unwrap();

                let from_ty = from.as_ref().unwrap_or(ty);

                let from_ty = if default {
                    quote! {Option::<#from_ty>}
                } else {
                    quote! {#from_ty}
                };

                let absent_key = if is_opt || default {
                    quote! {None}
                } else {
                    quote! { return Err(::winnow::error::ErrMode::Backtrack(::winnow::error::ContextError::new())) }
                };

                let mut extraction = if flatten {
                    quote! {
                        let #name = #from_ty::from_raw_dict(dict)?;
                    }
                } else {
                    quote! {
                    let #name: #from_ty = if let Some(value) = dict.pop(&#field_str.into()) {
                        value.extract()?
                    } else {
                        #absent_key
                    };

                    }
                };

                if default {
                    extraction = quote! {
                        #extraction
                        let #name = #name.unwrap_or_default();
                    }
                }

                if from.is_some() {
                    extraction = quote! {
                        #extraction
                        let #name: #ty = #name.into();
                    }
                }
                extraction
            });

            quote! {
                #(
                    #field_by_field
                )*

                let res = Self {
                    #(
                        #fieldname,
                    )*
                };
            }
        }
        _ => unimplemented!(),
    }
}
