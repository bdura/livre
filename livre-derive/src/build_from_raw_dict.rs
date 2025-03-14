use std::collections::HashSet;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    parse_macro_input, parse_quote, parse_quote_spanned, Data, DataStruct, DeriveInput, Fields,
    GenericParam, Generics,
};

use crate::utilities::extraction;

fn add_extraction_trait_bounds(mut generics: Generics, flattened: HashSet<String>) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            if flattened.contains(&type_param.ident.to_string()) {
                type_param
                    .bounds
                    .push(parse_quote!(crate::follow_refs::BuildFromRawDict));
            } else {
                type_param
                    .bounds
                    .push(parse_quote!(crate::follow_refs::Build));
            }
        }
    }
    generics
}

pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parse the input tokens into a syntax tree.
    let input = parse_macro_input!(input as DeriveInput);

    // Used in the quasi-quotation below as `#name`.
    let name = input.ident;

    let (extraction, flattened) = generate_extraction(&input.data);

    // Add a bound `T: Extract` to every type parameter T.
    let generics = add_extraction_trait_bounds(input.generics, flattened);
    let mut gen_lt = generics.clone();

    gen_lt
        .params
        .push(parse_quote_spanned!(Span::mixed_site() => 'de));

    let (_, ty_generics, where_clause) = generics.split_for_impl();
    let (impl_generics, _, _) = gen_lt.split_for_impl();

    let expanded = quote! {
        impl #impl_generics crate::follow_refs::BuildFromRawDict for #name #ty_generics #where_clause {
            fn build_from_raw_dict<B>(dict: &mut crate::extraction::RawDict<'_>, builder: &B) -> ::winnow::PResult<Self>
            where
                B: crate::follow_refs::Builder,
            {
                #extraction
                Ok(res)
            }
        }
    };

    // Hand the output tokens back to the compiler.
    proc_macro::TokenStream::from(expanded)
}

fn generate_extraction(data: &Data) -> (TokenStream, HashSet<String>) {
    let mut set = HashSet::new();
    match data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => {
            let fieldname = fields.named.iter().map(|f| &f.ident).collect::<Vec<_>>();

            let field_by_field = fields.named.iter().map(|f| {
                extraction::extract_field(f, &mut set, quote! {build_from_raw_dict}, quote! {build})
            });

            let extraction = quote! {
                #(
                    #field_by_field
                )*

                let res = Self {
                    #(
                        #fieldname,
                    )*
                };
            };

            (extraction, set)
        }
        _ => unimplemented!(),
    }
}
