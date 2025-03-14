use syn::{ExprClosure, Field, LitStr, Result, Type};

use super::option;

pub struct Attributes {
    pub field_str: String,
    pub from: Option<Type>,
    pub flatten: bool,
    pub default: Option<DefaultValue>,
    pub is_opt: bool,
}

pub enum DefaultValue {
    None,
    Lit(LitStr),
}

/// Find the value of a #[livre] attribute.
pub fn parse_attributes(field: &Field) -> Result<Attributes> {
    let mut rename = None;
    let mut from = None;
    let mut flatten = false;
    let mut default = None;

    for attr in &field.attrs {
        if !attr.path().is_ident("livre") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("flatten") {
                flatten = true;
                return Ok(());
            }

            if meta.path.is_ident("rename") {
                let s: LitStr = meta.value()?.parse()?;
                if rename.is_some() {
                    return Err(meta.error("duplicate rename attribute"));
                }
                rename = Some(s.value());

                return Ok(());
            }

            if meta.path.is_ident("from") {
                let s: Type = meta.value()?.parse()?;
                if from.is_some() {
                    return Err(meta.error("duplicate from attribute"));
                }
                from = Some(s);

                return Ok(());
            }

            if meta.path.is_ident("default") {
                if let Ok(inner) = meta.value() {
                    if let Ok(lit) = inner.parse() {
                        default = Some(DefaultValue::Lit(lit))
                    } else {
                        default = Some(DefaultValue::None);
                    }
                }
                return Ok(());
            }

            Err(meta.error("unsupported attribute"))
        })?;
    }

    let is_opt = option::is_option(&field.ty);

    let field_str = rename.unwrap_or_else(|| {
        let name = field.ident.as_ref().unwrap().to_string();
        pascal_case(&name)
    });

    Ok(Attributes {
        field_str,
        from,
        flatten,
        default,
        is_opt,
    })
}

fn pascal_case(s: &str) -> String {
    s.split('_').map(capitalize).collect()
}

fn capitalize(s: &str) -> String {
    let (c, rest) = s.split_at(1);
    let c = c.to_uppercase();

    [&c, rest].join("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capitalization() {
        assert_eq!(capitalize("n"), "N");
        assert_eq!(capitalize("next"), "Next");
    }
}
