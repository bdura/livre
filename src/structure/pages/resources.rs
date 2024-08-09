use std::collections::HashMap;

use crate::{fonts::FontTransient, parsers::TypedReference};
use serde::Deserialize;

#[derive(Debug, PartialEq, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Resources {
    // pub ext_g_state: Option<&'i [u8]>,
    // pub color_space: Option<&'i [u8]>,
    // pub pattern: Option<&'i [u8]>,
    // pub shading: Option<&'i [u8]>,
    // pub x_object: Option<&'i [u8]>,
    pub font: HashMap<String, TypedReference<FontTransient>>,
    // pub properties: Option<&'i [u8]>,
}

#[cfg(test)]
mod tests {

    use crate::serde::extract_deserialize;
    use indoc::indoc;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(
        indoc!{b"
            <</ProcSet [/PDF /Text /ImageB /ImageC /ImageI]
            /ExtGState <</G3 3 0 R>>
            /Font <</F4 4 0 R>>>>
        "},
        Resources { font: HashMap::from([("F4".into(), TypedReference::new(4, 0))]) }
    )]
    fn resources(#[case] input: &[u8], #[case] expected: Resources) {
        let (_, resources) = extract_deserialize(input).unwrap();
        assert_eq!(expected, resources);
    }
}
