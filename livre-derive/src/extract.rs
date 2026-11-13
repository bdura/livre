//! Generator for the `Extract` derive: tuple structs, unit structs and enums.
//!
//! Every winnow call is fully qualified, so the expansion compiles whatever
//! the caller has imported.

use std::collections::HashSet;

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{parse_quote, parse_quote_spanned, DeriveInput, Ident};

use crate::{generics::add_bounds, plan::ContainerPlan};

/// Expands `#[derive(Extract)]`, or returns the plan's spanned error for named
/// structs, unions and `#[livre]` field attributes.
pub fn derive(input: DeriveInput) -> syn::Result<TokenStream> {
    let plan = ContainerPlan::for_extract(&input)?;

    let name = &input.ident;
    // Mixed-site, so it neither captures nor is captured by user identifiers.
    let input_ident = Ident::new("input", Span::mixed_site());

    let extraction = generate_extraction(&input_ident, &plan);

    // Add a bound `T: Extract` to every type parameter T.
    let generics = add_bounds(
        input.generics.clone(),
        &HashSet::new(),
        parse_quote!(::livre::extraction::Extract<'de>),
        parse_quote!(::livre::extraction::Extract<'de>),
    );
    let mut gen_lt = generics.clone();

    gen_lt
        .params
        .push(parse_quote_spanned!(Span::mixed_site() => 'de));

    let (_, ty_generics, where_clause) = generics.split_for_impl();
    let (impl_generics, _, _) = gen_lt.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::livre::extraction::Extract<'de> for #name #ty_generics #where_clause {
            fn extract(#input_ident: &mut &'de ::winnow::stream::BStr) -> ::winnow::ModalResult<Self> {
                #extraction
            }
        }
    })
}

/// Body of `extract`, reading from the `&mut &BStr` binding `input`.
fn generate_extraction(input: &Ident, plan: &ContainerPlan) -> TokenStream {
    match plan {
        ContainerPlan::Tuple { arity } => {
            let field_by_field: Vec<Ident> = (0..*arity)
                .map(|i| format_ident!("t{}", i, span = Span::mixed_site()))
                .collect();

            quote! {
                let (#(#field_by_field,)*) = ::livre::extraction::extract(#input)?;
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
                quote! {
                    ::winnow::Parser::map(::livre::extraction::extract, Self::#ident),
                }
            });

            quote! {
                ::winnow::Parser::parse_next(
                    &mut ::winnow::combinator::alt((#(#alternatives)*)),
                    #input,
                )
            }
        }
        // `for_extract` never builds a dictionary plan.
        ContainerPlan::Dict { .. } => quote! {},
    }
}
