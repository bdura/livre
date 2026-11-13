use std::collections::HashSet;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{parse_quote_spanned, DeriveInput};

use crate::{
    add_extraction_trait_bounds,
    dict::extract_field,
    plan::{ContainerPlan, FieldPlan},
};

pub fn derive(input: DeriveInput) -> syn::Result<TokenStream> {
    let ContainerPlan::Dict { fields } = ContainerPlan::for_dict(&input, "FromRawDict")? else {
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

    // Add a bound `T: Extract` to every type parameter T.
    let generics = add_extraction_trait_bounds(input.generics, flattened);
    let mut gen_lt = generics.clone();

    gen_lt
        .params
        .push(parse_quote_spanned!(Span::mixed_site() => 'de));

    let (_, ty_generics, where_clause) = generics.split_for_impl();
    let (impl_generics, _, _) = gen_lt.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::livre::extraction::FromRawDict<'de> for #name #ty_generics #where_clause {
            fn from_raw_dict(dict: &mut ::livre::extraction::RawDict<'de>) -> ::winnow::ModalResult<Self> {
                #extraction
                Ok(res)
            }
        }
    })
}

fn generate_extraction(fields: &[FieldPlan]) -> TokenStream {
    let fieldname = fields.iter().map(|f| &f.ident);
    let field_by_field = fields
        .iter()
        .map(|f| extract_field(f, quote! {from_raw_dict(dict)}, quote! {extract()}));

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
