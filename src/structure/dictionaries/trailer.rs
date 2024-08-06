use crate::parsers::TypedReference;
use serde::Deserialize;

use super::Catalogue;

use super::Info;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TrailerDict {
    /// The total number of entries in the PDF file's cross-ref table,
    /// as defined by the combination of the original section and all
    /// update sections. Equivalently, this value shall be 1 greater
    /// than the highest object number defined in the PDF file.
    /// 
    /// Any object in a cross-reference section whose number is greater than
    /// this value shall be ignored and defined to be missing by a PDF reader.
    pub size: usize,
    /// The byte offset from the beginning of the PDF file to the beginning
    /// of the previous cross-reference stream.
    pub prev: Option<usize>,
    ///  The [catalog dictionary](Catalogue) for the PDF file.
    pub root: TypedReference<Catalogue>,
    // pub encrypt: Encrypt,
    /// The PDF file’s [information dictionary](Info).
    pub info: TypedReference<Info>,
    // #[livre(rename = "ID")]
    // TODO: change to HexString!
    // pub id: MaybeArray<HexBytes>,
}

#[cfg(test)]
mod tests {

    use std::fmt::Debug;

    use indoc::indoc;
    use crate::serde::{from_bytes, MaybeArray};
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
