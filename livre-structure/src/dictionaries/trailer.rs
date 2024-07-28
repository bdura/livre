use livre_extraction::TypedReference;
use serde::Deserialize;

use crate::Catalogue;

use super::Info;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TrailerDict {
    pub size: usize,
    pub prev: Option<usize>,
    pub root: TypedReference<Catalogue>,
    // pub encrypt: Encrypt,
    pub info: TypedReference<Info>,
    // #[livre(rename = "ID")]
    // TODO: change to HexString!
    // pub id: MaybeArray<HexBytes>,
}

#[cfg(test)]
mod tests {

    use std::fmt::Debug;

    use indoc::indoc;
    use livre_serde::{from_bytes, MaybeArray};
    use rstest::rstest;

    use super::*;

    #[derive(Debug, PartialEq, Deserialize)]
    #[serde(rename_all="PascalCase")]
    struct Wrapper<T> {
        #[serde(default)]
        filter: MaybeArray<()>,
        #[serde(flatten)]
        inner: T,
    }

    #[derive(Debug, PartialEq, Deserialize)]
    #[serde(rename_all="PascalCase")]
    struct Wrapped<T> {
        length: usize,
        #[serde(flatten)]
        inner: T,
    }

    #[rstest]
    #[case(
        indoc!{b"
            <</Size 92813
            /Root 90794 0 R
            /Info 90792 0 R
            /ID[<2B551D2AFE52654494F9720283CFF1C4><6cdabf5b33a08c969604fab8979c5412>]
            /Prev 116
            /Type/XRef
            /W[ 1 3 0]
            /Index[ 1 1 7 1 14 1 16 1 91807 1006]
            /Length 1>>\
        "},
        TrailerDict{ 
            size: 92813, 
            prev: Some(116), 
            root: TypedReference::new(90794, 0), 
            info: TypedReference::new(90792, 0), 
        }
    )]
    #[case(
        indoc!{b"
            <</Size 92813
            /Root 90794 0 R
            /Info 90792 0 R
            /ID[<2B551D2AFE52654494F9720283CFF1C4><6cdabf5b33a08c969604fab8979c5412>]
            /Type/XRef
            /W[ 1 3 0]
            /Index[ 1 1 7 1 14 1 16 1 91807 1006]
            /Length 1>>\
        "},
        Wrapper {
            filter: MaybeArray(vec![]),
            inner: Wrapped{ 
                length: 1, 
                    inner: TrailerDict {
                    size: 92813, 
                    prev: None, 
                    root: TypedReference::new(90794, 0), 
                    info: TypedReference::new(90792, 0), 
                }
            }
        }
    )]
    fn deserialize<'de, T>(#[case] input: &'de [u8], #[case] expected: T)
    where
        T: Deserialize<'de> + Debug + PartialEq,
    {
        let result = from_bytes(input).unwrap();
        // let (_, result) = extract_deserialize(input).unwrap();
        assert_eq!(expected, result);
    }
}
