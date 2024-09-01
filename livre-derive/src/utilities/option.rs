use syn::{Path, PathSegment, Type};

fn extract_type_path(ty: &Type) -> Option<&Path> {
    match *ty {
        syn::Type::Path(ref typepath) if typepath.qself.is_none() => Some(&typepath.path),
        _ => None,
    }
}

fn extract_option_segment(path: &Path) -> Option<&PathSegment> {
    let idents_of_path = path.segments.iter().fold(String::new(), |mut acc, v| {
        acc.push_str(&v.ident.to_string());
        acc.push('|');
        acc
    });
    vec!["Option|", "std|option|Option|", "core|option|Option|"]
        .into_iter()
        .find(|s| idents_of_path == *s)
        .and_then(|_| path.segments.last())
}

fn get_option_segment(ty: &Type) -> Option<&PathSegment> {
    let path = extract_type_path(ty)?;
    extract_option_segment(path)
}

pub fn is_option(ty: &Type) -> bool {
    get_option_segment(ty).is_some()
}
