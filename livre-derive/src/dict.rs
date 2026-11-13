//! Code generation for the dictionary derives, `FromRawDict` and `BuildFromRawDict`.
//!
//! Both derives share one generator; they differ only by the trait they
//! implement and how a value is read, which [`DictTraits`] captures.

use std::collections::HashSet;

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{parse_quote, DeriveInput, Ident, Lifetime, LifetimeParam, Path, Type};

use crate::{
    attrs::DefaultValue,
    generics::add_bounds,
    plan::{ContainerPlan, FieldPlan, Mode},
};

/// Hygienic names available to the generated method body.
pub struct Locals {
    /// The `&mut RawDict` parameter.
    pub dict: Ident,
    /// The builder parameter; only bound by `BuildFromRawDict`.
    pub builder: Ident,
    /// The `RawValue` popped from the dictionary.
    pub value: Ident,
    /// An error being labelled with its key.
    pub err: Ident,
    /// The input lifetime; only declared when [`DictTraits::needs_de_lifetime`].
    pub de: Lifetime,
}

impl Locals {
    fn new() -> Self {
        let span = Span::mixed_site();
        Self {
            dict: format_ident!("dict", span = span),
            builder: format_ident!("builder", span = span),
            value: format_ident!("value", span = span),
            err: format_ident!("err", span = span),
            de: Lifetime::new("'de", span),
        }
    }
}

/// What distinguishes one dictionary derive from the other.
pub struct DictTraits {
    /// The derive's name, for diagnostics.
    pub name: &'static str,
    /// The implemented trait.
    pub trait_path: fn(&Locals) -> Path,
    /// The trait method, signature and all, wrapping the given body.
    pub method: fn(&Locals, TokenStream) -> TokenStream,
    /// Reads `locals.value`, a `RawValue`, as the given type.
    pub simple_call: fn(&Locals, &TokenStream) -> TokenStream,
    /// Delegates a flattened field to its type, on the same dictionary.
    pub flatten_call: fn(&Locals) -> TokenStream,
    /// Bound on type parameters read as values.
    pub simple_bound: fn(&Locals) -> Path,
    /// Bound on type parameters that a field flattens.
    pub flatten_bound: fn(&Locals) -> Path,
    /// Whether the impl declares the `'de` input lifetime.
    pub needs_de_lifetime: bool,
}

/// `FromRawDict<'de>`: values are extracted, borrowing from the input.
pub const FROM_RAW_DICT: DictTraits = DictTraits {
    name: "FromRawDict",
    trait_path: |l| {
        let de = &l.de;
        parse_quote!(::livre::extraction::FromRawDict<#de>)
    },
    method: |l, body| {
        let Locals { dict, de, .. } = l;
        quote! {
            fn from_raw_dict(
                #dict: &mut ::livre::extraction::RawDict<#de>,
            ) -> ::winnow::ModalResult<Self> {
                #body
            }
        }
    },
    simple_call: |l, ty| {
        let value = &l.value;
        quote!(#value.extract::<#ty>())
    },
    flatten_call: |l| {
        let dict = &l.dict;
        quote!(from_raw_dict(#dict))
    },
    simple_bound: |l| {
        let de = &l.de;
        parse_quote!(::livre::extraction::Extract<#de>)
    },
    flatten_bound: |l| {
        let de = &l.de;
        parse_quote!(::livre::extraction::FromRawDict<#de>)
    },
    needs_de_lifetime: true,
};

/// `BuildFromRawDict`: values are built, following references through the builder.
pub const BUILD_FROM_RAW_DICT: DictTraits = DictTraits {
    name: "BuildFromRawDict",
    trait_path: |_| parse_quote!(::livre::follow_refs::BuildFromRawDict),
    method: |l, body| {
        let Locals { dict, builder, .. } = l;
        quote! {
            fn build_from_raw_dict<B>(
                #dict: &mut ::livre::extraction::RawDict<'_>,
                #builder: &B,
            ) -> ::winnow::ModalResult<Self>
            where
                B: ::livre::follow_refs::Builder,
            {
                #body
            }
        }
    },
    simple_call: |l, ty| {
        let Locals { value, builder, .. } = l;
        quote!(#value.build::<#ty, _>(#builder))
    },
    flatten_call: |l| {
        let Locals { dict, builder, .. } = l;
        quote!(build_from_raw_dict(#dict, #builder))
    },
    simple_bound: |_| parse_quote!(::livre::follow_refs::Build),
    flatten_bound: |_| parse_quote!(::livre::follow_refs::BuildFromRawDict),
    needs_de_lifetime: false,
};

/// Derives the trait described by `traits` on a struct with named fields.
pub fn derive(input: DeriveInput, traits: &DictTraits) -> syn::Result<TokenStream> {
    let ContainerPlan::Dict { fields } = ContainerPlan::for_dict(&input, traits.name)? else {
        return Err(syn::Error::new(
            Span::call_site(),
            "internal error: `for_dict` returned a non-dictionary plan",
        ));
    };

    let locals = Locals::new();
    let name = &input.ident;

    let flattened: HashSet<Ident> = fields
        .iter()
        .filter_map(|f| f.flattened_param.clone())
        .collect();
    let generics = add_bounds(
        input.generics.clone(),
        &flattened,
        (traits.simple_bound)(&locals),
        (traits.flatten_bound)(&locals),
    );
    let mut impl_generics = generics.clone();
    if traits.needs_de_lifetime {
        impl_generics
            .params
            .push(LifetimeParam::new(locals.de.clone()).into());
    }
    let (_, ty_generics, where_clause) = generics.split_for_impl();
    let (impl_generics, _, _) = impl_generics.split_for_impl();

    let trait_path = (traits.trait_path)(&locals);

    // Fields are bound to hygienic locals so that a `default` expression
    // cannot observe, or be shadowed by, another field.
    let bindings: Vec<Ident> = (0..fields.len())
        .map(|i| format_ident!("field_{}", i, span = Span::mixed_site()))
        .collect();
    let statements = fields
        .iter()
        .zip(&bindings)
        .map(|(field, binding)| field_statement(field, binding, traits, &locals));
    let idents = fields.iter().map(|f| &f.ident);

    let body = quote! {
        #(#statements)*
        ::core::result::Result::Ok(Self {
            #(#idents: #bindings,)*
        })
    };
    let method = (traits.method)(&locals, body);

    Ok(quote! {
        impl #impl_generics #trait_path for #name #ty_generics #where_clause {
            #method
        }
    })
}

/// The statement binding one field's value to `binding`.
///
/// A keyed field pops its key into an `Option` of the read type, converts it
/// with `Into` when `from` is set, then resolves absence according to its
/// mode; the default is therefore typed as the field.
fn field_statement(
    field: &FieldPlan,
    binding: &Ident,
    traits: &DictTraits,
    locals: &Locals,
) -> TokenStream {
    let ty = &field.ty;
    let Locals {
        dict, value, err, ..
    } = locals;

    let key = match (&field.mode, &field.key) {
        (Mode::Flatten, _) | (_, None) => {
            let trait_path = (traits.trait_path)(locals);
            let call = (traits.flatten_call)(locals);
            return quote! {
                let #binding = <#ty as #trait_path>::#call?;
            };
        }
        (_, Some(key)) => key,
    };

    // Design §6.3: errors carry the PDF key, slash included, and nothing else.
    let label = format!("/{key}");
    let label = quote!(::winnow::error::StrContext::Label(#label));

    // The value read when the key is present. An `Option` field reads
    // `Option<F>` so that a `null` value gives `None`.
    let labelled = |read_ty: TokenStream| {
        let call = (traits.simple_call)(locals, &read_ty);
        // `ErrMode::map` keeps `Backtrack` and `Cut` apart.
        quote! {
            #call.map_err(|#err| #err.map(|mut #err| {
                #err.push(#label);
                #err
            }))?
        }
    };
    let read = match (&field.mode, &field.from) {
        (Mode::Optional, None) => labelled(quote!(#ty)),
        (Mode::Optional, Some(from)) => labelled(quote!(::core::option::Option<#from>)),
        (_, from) => {
            let read_ty: &Type = from.as_ref().unwrap_or(ty);
            let read = labelled(quote!(#read_ty));
            quote!(::core::option::Option::Some(#read))
        }
    };

    let convert = field
        .from
        .as_ref()
        .map(|_| quote!(.map(::core::convert::Into::into)));

    let resolve = match &field.mode {
        Mode::Required => quote! {
            .ok_or_else(|| {
                let mut #err = ::winnow::error::ContextError::new();
                #err.push(#label);
                ::winnow::error::ErrMode::Backtrack(#err)
            })?
        },
        Mode::Default(default) => default_call(default),
        Mode::Optional | Mode::Flatten => quote!(),
    };

    quote! {
        let #binding: #ty = match #dict.pop(&::livre::extraction::Name::from(#key)) {
            ::core::option::Option::Some(#value) => #read,
            ::core::option::Option::None => ::core::option::Option::None,
        }
        #convert
        #resolve;
    }
}

/// The method call that fills an absent value from a `default` attribute.
fn default_call(default: &DefaultValue) -> TokenStream {
    match default {
        DefaultValue::Bare => quote!(.unwrap_or_default()),
        DefaultValue::Lit(lit) => quote!(.unwrap_or(#lit)),
        DefaultValue::Closure(closure) => quote!(.unwrap_or_else(#closure)),
        DefaultValue::Expr(expr) => quote!(.unwrap_or_else(|| #expr)),
    }
}
