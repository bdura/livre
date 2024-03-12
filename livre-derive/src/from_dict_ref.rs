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

    // Generate an expression to sum up the heap size of each field.
    let extraction = generate_extraction(&input.data);

    let expanded = quote! {
        // The generated impl.
        impl<'input> ::livre_extraction::FromDictRef<'input> for #name #ty_generics #where_clause {
            fn from_dict_ref(dict: &mut ::livre_extraction::RawDict<'input>) -> ::livre_extraction::error::Result<Self> {
                use ::nom::error::ParseError;

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
                    field_str,
                    is_opt,
                } = attr::parse_attributes(f).unwrap();

                if flatten {
                    quote! {
                        let #name = #ty::from_dict_ref(dict)?;
                    }
                } else {
                    let func = if is_opt {
                        quote! {pop_opt}
                    } else {
                        quote! {pop}
                    };

                    quote! {
                        let #name: #ty = dict.#func(#field_str)?;
                    }
                }
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
