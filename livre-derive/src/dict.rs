//! Field-level code generation shared by the dictionary derives.

use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    attrs::DefaultValue,
    plan::{FieldPlan, Mode},
};

/// The method call that fills an absent value from a `default` attribute.
fn default_call(default: &DefaultValue) -> TokenStream {
    match default {
        DefaultValue::Bare => quote! {unwrap_or_default()},
        DefaultValue::Lit(lit) => quote! {unwrap_or(#lit)},
        DefaultValue::Closure(closure) => quote! {unwrap_or_else(#closure)},
        DefaultValue::Expr(expr) => quote! {unwrap_or_else(|| #expr)},
    }
}

/// Generates the statements binding one field, as a local named after it.
///
/// `complex` is the method call delegating a flattened field to its type
/// (e.g. `from_raw_dict(dict)`); `simple` extracts a single popped value
/// (e.g. `extract()`).
pub fn extract_field(plan: &FieldPlan, complex: TokenStream, simple: TokenStream) -> TokenStream {
    let name = &plan.ident;
    let ty = &plan.ty;

    let (key, default, is_opt) = match &plan.mode {
        Mode::Flatten => {
            return quote! {
                let #name = <#ty>::#complex?;
            };
        }
        Mode::Optional => (&plan.key, None, true),
        Mode::Default(default) => (&plan.key, Some(default), false),
        Mode::Required => (&plan.key, None, false),
    };

    let from_ty = plan.from.as_ref().unwrap_or(ty);

    // The default is applied before the `from` conversion, on the source type.
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

    let mut extraction = quote! {
        let #name: #from_ty = if let Some(value) = dict.pop(&#key.into()) {
            value.#simple?
        } else {
            #absent_key
        };
    };

    if let Some(default) = default {
        let default = default_call(default);
        extraction = quote! {
            #extraction
            let #name = #name.#default;
        }
    }

    if plan.from.is_some() {
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
