//! Analysed view of a derive input: what each field does and under which key.
//!
//! Every validation rule on attribute combinations and container shapes is
//! enforced here, so generators can trust the plan they receive.

use syn::{
    ext::IdentExt, Data, DataEnum, DataStruct, DataUnion, DeriveInput, Field, Fields,
    GenericArgument, Generics, Ident, PathArguments, Type,
};

use crate::attrs::{DefaultValue, FieldAttrs};

/// How a dictionary field is filled.
pub enum Mode {
    /// Parsed from the same dictionary as the container; has no key.
    Flatten,
    /// The field is an `Option<T>`: `None` when the key is absent.
    Optional,
    /// The given default when the key is absent.
    Default(DefaultValue),
    /// The key must be present.
    Required,
}

/// One named field of a dictionary struct, with its key and mode resolved.
pub struct FieldPlan {
    pub ident: Ident,
    pub ty: Type,
    /// The PDF dictionary key, without its slash. `None` only for [`Mode::Flatten`].
    pub key: Option<String>,
    /// The type to extract before converting with `Into`.
    pub from: Option<Type>,
    pub mode: Mode,
    /// The container's type parameter this field flattens, when its type is
    /// exactly that bare parameter (e.g. `T`, but not `Inner<T>`).
    pub flattened_param: Option<Ident>,
}

impl FieldPlan {
    /// Resolves a named field's attributes into a plan.
    ///
    /// Rejects `flatten` combined with any other attribute, `default` on an
    /// `Option` field and an empty `rename`.
    pub fn analyse(field: &Field, generics: &Generics) -> syn::Result<Self> {
        let ident = field
            .ident
            .clone()
            .ok_or_else(|| syn::Error::new_spanned(field, "expected a named field"))?;
        let attrs = FieldAttrs::parse(&field.attrs)?;
        let ty = field.ty.clone();

        if attrs.flatten.is_some() {
            let conflict = [attrs.spans.rename, attrs.spans.from, attrs.spans.default]
                .into_iter()
                .flatten()
                .next();
            if let Some(span) = conflict {
                return Err(syn::Error::new(
                    span,
                    "`flatten` cannot be combined with other attributes",
                ));
            }

            let flattened_param = bare_type_param(&ty, generics);
            return Ok(Self {
                ident,
                ty,
                key: None,
                from: None,
                mode: Mode::Flatten,
                flattened_param,
            });
        }

        let key = match &attrs.rename {
            Some(lit) => {
                let value = lit.value();
                if value.is_empty() {
                    return Err(syn::Error::new(lit.span(), "renamed key must not be empty"));
                }
                value
            }
            None => pascal_case(&ident.unraw().to_string()),
        };

        let mode = match (attrs.default, is_option(&ty)) {
            (Some(_), true) => {
                let span = attrs.spans.default.unwrap_or_else(|| ident.span());
                return Err(syn::Error::new(
                    span,
                    "`default` is redundant on an `Option` field",
                ));
            }
            (Some(default), false) => Mode::Default(default),
            (None, true) => Mode::Optional,
            (None, false) => Mode::Required,
        };

        Ok(Self {
            ident,
            ty,
            key: Some(key),
            from: attrs.from,
            mode,
            flattened_param: None,
        })
    }
}

/// The supported shapes of a derive input.
pub enum ContainerPlan {
    Dict { fields: Vec<FieldPlan> },
    Tuple { arity: usize },
    Unit,
    Enum { variants: Vec<Ident> },
}

impl ContainerPlan {
    /// Plans a `FromRawDict` or `BuildFromRawDict` derive, named by `trait_name`
    /// in diagnostics. Only structs with named fields are accepted.
    pub fn for_dict(input: &DeriveInput, trait_name: &str) -> syn::Result<Self> {
        match &input.data {
            Data::Struct(DataStruct {
                fields: Fields::Named(fields),
                ..
            }) => {
                let fields = fields
                    .named
                    .iter()
                    .map(|f| FieldPlan::analyse(f, &input.generics))
                    .collect::<syn::Result<_>>()?;
                Ok(Self::Dict { fields })
            }
            _ => Err(syn::Error::new(
                input.ident.span(),
                format!("`{trait_name}` can only be derived on a struct with named fields"),
            )),
        }
    }

    /// Plans an `Extract` derive: tuple struct, unit struct or enum.
    ///
    /// Named structs are refused with a pointer to `FromRawDict`, and no
    /// field may carry a `#[livre]` attribute.
    pub fn for_extract(input: &DeriveInput) -> syn::Result<Self> {
        match &input.data {
            Data::Struct(DataStruct {
                fields: Fields::Named(_),
                ..
            }) => Err(syn::Error::new(
                input.ident.span(),
                "derive `FromRawDict` instead; it provides `Extract` through a blanket impl",
            )),
            Data::Struct(DataStruct {
                fields: Fields::Unnamed(fields),
                ..
            }) => {
                reject_livre_attrs(fields.unnamed.iter())?;
                Ok(Self::Tuple {
                    arity: fields.unnamed.len(),
                })
            }
            Data::Struct(DataStruct {
                fields: Fields::Unit,
                ..
            }) => Ok(Self::Unit),
            Data::Enum(DataEnum { variants, .. }) => {
                reject_livre_attrs(variants.iter().flat_map(|v| v.fields.iter()))?;
                Ok(Self::Enum {
                    variants: variants.iter().map(|v| v.ident.clone()).collect(),
                })
            }
            Data::Union(DataUnion { union_token, .. }) => Err(syn::Error::new(
                union_token.span,
                "unions are not supported",
            )),
        }
    }
}

fn reject_livre_attrs<'a>(fields: impl Iterator<Item = &'a Field>) -> syn::Result<()> {
    for field in fields {
        if let Some(attr) = field.attrs.iter().find(|a| a.path().is_ident("livre")) {
            return Err(syn::Error::new_spanned(
                attr,
                "attributes are not supported on `Extract` fields",
            ));
        }
    }
    Ok(())
}

/// The type parameter of `generics` that `ty` names, if `ty` is exactly that
/// bare, single-segment path.
fn bare_type_param(ty: &Type, generics: &Generics) -> Option<Ident> {
    let Type::Path(path) = ty else { return None };
    if path.qself.is_some() {
        return None;
    }
    let ident = path.path.get_ident()?;
    generics
        .type_params()
        .any(|p| p.ident == *ident)
        .then(|| ident.clone())
}

/// Whether `ty` is spelt `Option<T>`, `std::option::Option<T>` or
/// `core::option::Option<T>`.
fn is_option(ty: &Type) -> bool {
    let Type::Path(path) = ty else { return false };
    if path.qself.is_some() {
        return false;
    }
    let segments: Vec<String> = path
        .path
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect();
    let segments: Vec<&str> = segments.iter().map(String::as_str).collect();
    let is_option_path = matches!(
        segments.as_slice(),
        ["Option"] | ["std", "option", "Option"] | ["core", "option", "Option"]
    );
    is_option_path
        && path.path.segments.last().is_some_and(|s| {
            matches!(
                &s.arguments,
                PathArguments::AngleBracketed(args)
                    if args.args.len() == 1 && matches!(args.args[0], GenericArgument::Type(_))
            )
        })
}

/// `snake_case` to `PascalCase`: empty segments are skipped and the first
/// `char` of each remaining segment is uppercased.
fn pascal_case(s: &str) -> String {
    s.split('_')
        .filter(|seg| !seg.is_empty())
        .flat_map(|seg| {
            let mut chars = seg.chars();
            let first = chars.next().into_iter().flat_map(char::to_uppercase);
            first.chain(chars)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pascal_case_keys() {
        assert_eq!(pascal_case("n"), "N");
        assert_eq!(pascal_case("last_modified"), "LastModified");
        assert_eq!(pascal_case("__weird"), "Weird");
        assert_eq!(pascal_case("a__b_"), "AB");
        assert_eq!(pascal_case("_"), "");
    }
}
