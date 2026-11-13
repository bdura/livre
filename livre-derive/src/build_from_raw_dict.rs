use std::collections::HashSet;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{parse_quote, parse_quote_spanned, DeriveInput, GenericParam, Generics};

use crate::{
    dict::extract_field,
    plan::{ContainerPlan, FieldPlan},
};

fn add_extraction_trait_bounds(mut generics: Generics, flattened: HashSet<String>) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            if flattened.contains(&type_param.ident.to_string()) {
                type_param
                    .bounds
                    .push(parse_quote!(::livre::follow_refs::BuildFromRawDict));
            } else {
                type_param
                    .bounds
                    .push(parse_quote!(::livre::follow_refs::Build));
            }
        }
    }
    generics
}

pub fn derive(input: DeriveInput) -> syn::Result<TokenStream> {
    let ContainerPlan::Dict { fields } = ContainerPlan::for_dict(&input, "BuildFromRawDict")?
    else {
        return Err(syn::Error::new(
            Span::call_site(),
            "internal error: `for_dict` returned a non-dictionary plan",
        ));
    };

    let name = input.ident;

    let flattened: HashSet<String> = fields
        .iter()
        .filter_map(|f| f.flattened_param.as_ref().map(ToString::to_string))
        .collect();
    let extraction = generate_extraction(&fields);

    // Add a bound `T: Build` to every type parameter T.
    let generics = add_extraction_trait_bounds(input.generics, flattened);
    let mut gen_lt = generics.clone();

    gen_lt
        .params
        .push(parse_quote_spanned!(Span::mixed_site() => 'de));

    let (_, ty_generics, where_clause) = generics.split_for_impl();
    let (impl_generics, _, _) = gen_lt.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::livre::follow_refs::BuildFromRawDict for #name #ty_generics #where_clause {
            fn build_from_raw_dict<B>(dict: &mut ::livre::extraction::RawDict<'_>, builder: &B) -> ::winnow::ModalResult<Self>
            where
                B: ::livre::follow_refs::Builder,
            {
                #extraction
                Ok(res)
            }
        }
    })
}

fn generate_extraction(fields: &[FieldPlan]) -> TokenStream {
    let fieldname = fields.iter().map(|f| &f.ident);
    let field_by_field = fields.iter().map(|f| {
        extract_field(
            f,
            quote! {build_from_raw_dict(dict, builder)},
            quote! {build(builder)},
        )
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
