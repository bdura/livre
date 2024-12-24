use crate::extraction::{FromRawDict, HexadecimalString, MaybeArray, Reference};

use super::catalog::Catalog;

/// PDF file trailer.
///
/// The trailer of a PDF file enables a PDF processor to quickly find the cross-reference
/// table and certain special objects.
#[derive(Debug, Clone, PartialEq, FromRawDict)]
pub struct Trailer {
    /// From the specs:
    ///
    /// > The total number of entries in the PDF file's cross-ref table,
    /// > as defined by the combination of the original section and all
    /// > update sections. Equivalently, this value shall be 1 greater
    /// > than the highest object number defined in the PDF file.
    /// >
    /// > Any object in a cross-reference section whose number is greater than
    /// > this value shall be ignored and defined to be missing by a PDF reader.
    pub size: usize,
    /// The byte offset from the beginning of the PDF file to the beginning
    /// of the previous cross-reference stream.
    pub prev: Option<usize>,
    ///  > The [catalog dictionary](Catalogue) for the PDF file.
    ///
    /// NOTE: At this stage, we only keep track of the reference.
    /// TODO: generate a proper document from the sequence of trailer dicts
    pub root: Reference<Catalog>,
    // pub encrypt: Encrypt,
    // The PDF file’s [information dictionary](Info).
    // Not required.
    // pub info: Option<Reference<Info>>,
    #[livre(rename = "ID", from = Option<MaybeArray<HexadecimalString>>)]
    pub id: Option<Vec<HexadecimalString>>,
}


#[cfg(test)]
mod tests {

    use std::fmt::Debug;

    use indoc::indoc;
    use rstest::rstest;

    use crate::extraction::{extract, Extract};

    use super::*;

    #[rstest]
    #[case(
        indoc!{b"
            <</Size 92813
            /Root 90794 0 R
            /Info 90792 0 R
            /ID[<2B55><0a12>]
            /Prev 116
            /Type/XRef
            /W[ 1 3 0]
            /Index[ 1 1 7 1 14 1 16 1 91807 1006]
            /Length 1>>\
        "},
        Trailer{ 
            size: 92813, 
            prev: Some(116), 
            id: Some(vec![[0x2b, 0x55].into(), [0x0a, 0x12].into()]),
            root: Reference::from((90794, 0)), 
            //info: TypedReference::new(90792, 0), 
        }
    )]
    fn extraction<'de, T>(#[case] input: &'de [u8], #[case] expected: T)
    where
        T: Extract<'de> + Debug + PartialEq,
    {
        let result = extract(&mut input.as_ref()).unwrap();
        // let (_, result) = extract_deserialize(input).unwrap();
        assert_eq!(expected, result);
    }
}
