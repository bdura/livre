use std::collections::HashSet;

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{parse_quote_spanned, DeriveInput, Ident};

use crate::{add_extraction_trait_bounds, plan::ContainerPlan};

pub fn derive(input: DeriveInput) -> syn::Result<TokenStream> {
    let plan = ContainerPlan::for_extract(&input)?;

    let name = &input.ident;

    let extraction = generate_extraction(name, &plan);

    // Add a bound `T: Extract` to every type parameter T.
    let generics = add_extraction_trait_bounds(input.generics.clone(), HashSet::new());
    let mut gen_lt = generics.clone();

    gen_lt
        .params
        .push(parse_quote_spanned!(Span::mixed_site() => 'de));

    let (_, ty_generics, where_clause) = generics.split_for_impl();
    let (impl_generics, _, _) = gen_lt.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::livre::extraction::Extract<'de> for #name #ty_generics #where_clause {
            fn extract(input: &mut &'de ::winnow::stream::BStr) -> ::winnow::ModalResult<Self> {
                #extraction
            }
        }
    })
}

fn generate_extraction(name: &Ident, plan: &ContainerPlan) -> TokenStream {
    match plan {
        ContainerPlan::Tuple { arity } => {
            let field_by_field: Vec<Ident> = (0..*arity).map(|i| format_ident!("t{}", i)).collect();

            quote! {
                let (#(#field_by_field,)*) = ::livre::extraction::extract(input)?;
                Ok(Self(#(#field_by_field,)*))
            }
        }
        ContainerPlan::Unit => {
            quote! {
                Ok(Self)
            }
        }
        ContainerPlan::Enum { variants } => {
            let alternatives = variants.iter().map(|ident| {
                quote! {::livre::extraction::extract.map(#name::#ident),}
            });

            quote! {
                ::winnow::combinator::alt((
                    #(
                        #alternatives
                    )*
                )).parse_next(input)
            }
        }
        // `for_extract` never builds a dictionary plan.
        ContainerPlan::Dict { .. } => quote! {},
    }
}
