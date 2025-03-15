use std::collections::HashSet;

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, parse_quote_spanned, Data, DataEnum, DataStruct, DeriveInput, Fields, Ident,
};

use crate::add_extraction_trait_bounds;

pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parse the input tokens into a syntax tree.
    let input = parse_macro_input!(input as DeriveInput);

    // Used in the quasi-quotation below as `#name`.
    let name = &input.ident;

    let extraction = generate_extraction(&input);

    // Add a bound `T: Extract` to every type parameter T.
    let generics = add_extraction_trait_bounds(input.generics, HashSet::new());
    let mut gen_lt = generics.clone();

    gen_lt
        .params
        .push(parse_quote_spanned!(Span::mixed_site() => 'de));

    let (_, ty_generics, where_clause) = generics.split_for_impl();
    let (impl_generics, _, _) = gen_lt.split_for_impl();

    let expanded = quote! {
        // The generated impl.
        impl #impl_generics crate::extraction::Extract<'de> for #name #ty_generics #where_clause {
            fn extract(input: &mut &'de ::winnow::stream::BStr) -> ::winnow::PResult<Self> {
                #extraction
            }
        }
    };

    // Hand the output tokens back to the compiler.
    proc_macro::TokenStream::from(expanded)
}

fn generate_extraction(input: &DeriveInput) -> TokenStream {
    match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Unnamed(fields),
            ..
        }) => {
            let n = fields.unnamed.len();

            let field_by_field: Vec<Ident> = (0..n).map(|i| format_ident!("t{}", i)).collect();

            quote! {
                let (#(#field_by_field,)*) = crate::extraction::extract(input)?;
                Ok(Self(#(#field_by_field,)*))
            }
        }
        Data::Struct(DataStruct {
            fields: Fields::Unit,
            ..
        }) => {
            quote! {
                Ok(Self)
            }
        }
        Data::Enum(DataEnum { variants, .. }) => {
            let name = &input.ident;

            let alternatives = variants.iter().map(|v| {
                let ident = &v.ident;
                quote! {crate::extraction::extract.map(#name::#ident),}
            });

            quote! {
                ::winnow::combinator::alt((
                    #(
                        #alternatives
                    )*
                )).parse_next(input)
            }
        }
        _ => unimplemented!(),
    }
}
