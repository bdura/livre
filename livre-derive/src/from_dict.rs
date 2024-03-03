use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields};

use crate::add_trait_bounds;

use super::{attr, option};

pub fn derive_from_dict(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
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
        impl<'input> ::livre_extraction::FromDict<'input> for #name #ty_generics #where_clause {
            fn from_dict(mut dict: ::livre_extraction::RawDict<'input>) -> ::livre_extraction::error::Result<Self> {
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
            let fieldty = fields.named.iter().map(|f| &f.ty);
            let dictfunc = fields.named.iter().map(|f| {
                let is_opt = option::is_option(&f.ty);
                if is_opt {
                    quote!(pop_opt)
                } else {
                    quote!(pop)
                }
            });
            let fieldstr = fields
                .named
                .iter()
                .map(attr::name_of_field)
                .collect::<Result<Vec<_>, _>>()
                .unwrap();

            quote! {
                #(
                    let #fieldname: #fieldty = dict.#dictfunc(#fieldstr)?;
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
