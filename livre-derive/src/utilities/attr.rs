use syn::{Attribute, Field, LitStr, Result};

use super::option;

pub struct Attributes {
    pub field_str: String,
    pub flatten: bool,
    pub is_opt: bool,
}

/// Find the value of a #[livre(rename = "...")] attribute.
fn attr_rename(attrs: &[Attribute]) -> Result<Option<String>> {
    let mut rename = None;

    for attr in attrs {
        if !attr.path().is_ident("livre") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                let s: LitStr = meta.value()?.parse()?;
                if rename.is_some() {
                    return Err(meta.error("duplicate rename attribute"));
                }
                rename = Some(s.value());
            }
            Ok(())
        })?;
    }

    Ok(rename)
}

/// Find the value of a #[livre] attribute.
pub fn parse_attributes(field: &Field) -> Result<Attributes> {
    let mut rename = None;
    let mut flatten = false;

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

                Ok(())
            } else {
                Err(meta.error("unsupported attribute"))
            }
        })?;
    }

    let is_opt = option::is_option(&field.ty);

    let field_str = rename.unwrap_or_else(|| {
        let name = field.ident.as_ref().unwrap().to_string();
        pascal_case(&name)
    });

    Ok(Attributes {
        field_str,
        flatten,
        is_opt,
    })
}

/// Determine the name of a field, respecting a rename attribute.
///
/// The field is converted to PascalCase by default.
pub fn name_of_field(field: &Field) -> Result<String> {
    let rename = attr_rename(&field.attrs)?;
    Ok(rename.unwrap_or_else(|| {
        let name = field.ident.as_ref().unwrap().to_string();
        pascal_case(&name)
    }))
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
