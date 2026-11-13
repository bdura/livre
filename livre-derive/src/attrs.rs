//! Syntactic parse of the `#[livre(...)]` field attributes.
//!
//! This layer only records what was written and rejects what cannot be read:
//! unknown keys and repeated keys. Combination rules live in [`crate::plan`].

use proc_macro2::Span;
use syn::{Attribute, Expr, ExprClosure, Lit, LitStr, Token, Type};

/// The value of a `default` attribute, classified by how it is applied.
pub enum DefaultValue {
    /// `default`: use the type's `Default` impl.
    Bare,
    /// `default = <literal>`: used as is.
    Lit(Lit),
    /// `default = |...| ...`: called to produce the value.
    Closure(ExprClosure),
    /// `default = <expr>`: any other expression, evaluated only when needed.
    Expr(Expr),
}

/// Span of each attribute key, for diagnostics that point at the key itself.
#[derive(Default)]
pub struct AttrSpans {
    pub rename: Option<Span>,
    pub from: Option<Span>,
    pub default: Option<Span>,
}

/// Every `#[livre(...)]` key found on one field, each at most once.
#[derive(Default)]
pub struct FieldAttrs {
    pub rename: Option<LitStr>,
    pub from: Option<Type>,
    pub default: Option<DefaultValue>,
    /// Span of the `flatten` key, if present.
    pub flatten: Option<Span>,
    pub spans: AttrSpans,
}

impl FieldAttrs {
    /// Collects the keys of every `#[livre(...)]` attribute in `attrs`.
    ///
    /// Errors on an unknown key, or on a key given twice, even across
    /// separate `#[livre]` attributes.
    pub fn parse(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut out = Self::default();

        for attr in attrs {
            if !attr.path().is_ident("livre") {
                continue;
            }

            attr.parse_nested_meta(|meta| {
                let span = meta
                    .path
                    .get_ident()
                    .map_or_else(Span::call_site, |i| i.span());

                if meta.path.is_ident("flatten") {
                    if out.flatten.is_some() {
                        return Err(meta.error("duplicate `flatten` attribute"));
                    }
                    out.flatten = Some(span);
                    return Ok(());
                }

                if meta.path.is_ident("rename") {
                    if out.rename.is_some() {
                        return Err(meta.error("duplicate `rename` attribute"));
                    }
                    out.rename = Some(meta.value()?.parse()?);
                    out.spans.rename = Some(span);
                    return Ok(());
                }

                if meta.path.is_ident("from") {
                    if out.from.is_some() {
                        return Err(meta.error("duplicate `from` attribute"));
                    }
                    out.from = Some(meta.value()?.parse()?);
                    out.spans.from = Some(span);
                    return Ok(());
                }

                if meta.path.is_ident("default") {
                    if out.default.is_some() {
                        return Err(meta.error("duplicate `default` attribute"));
                    }
                    let value = if meta.input.peek(Token![=]) {
                        match meta.value()?.parse::<Expr>()? {
                            Expr::Lit(expr) => DefaultValue::Lit(expr.lit),
                            Expr::Closure(closure) => DefaultValue::Closure(closure),
                            expr => DefaultValue::Expr(expr),
                        }
                    } else {
                        DefaultValue::Bare
                    };
                    out.default = Some(value);
                    out.spans.default = Some(span);
                    return Ok(());
                }

                Err(meta.error("unsupported attribute"))
            })?;
        }

        Ok(out)
    }
}
