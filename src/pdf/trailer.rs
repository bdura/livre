use crate::{extraction::{FromRawDict, Reference}, Id};

/// PDF file trailer.
///
/// The trailer of a PDF file enables a PDF processor to quickly find the cross-reference
/// table and certain special objects. In the case of an updated PDF, the full trailer is
/// repeated (be it modified or not), hence only the last trailer of the document is necessary
/// for comprehension.
///
/// However, since previous cross-reference tables are linked together through the `prev`
/// key in the trailer, we still need to parse previous trailers.
/// A lighter version is availble for the specific case of iterating through cross-reference
/// tables.
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

    /// Reference to the PDF catalog.
    pub root: Reference<()>,

    // pub encrypt: Encrypt,
    // The PDF file’s [information dictionary](Info).
    // Not required.
    // pub info: Option<Reference<Info>>,

    /// The PDF identifier.
    ///
    /// From the specs:
    ///
    /// > Each PDF file identifier byte-string shall have a minimum length of 16 bytes.
    /// > If there is an Encrypt entry, this array and the two byte-strings shall be
    /// > direct objects and shall be unencrypted.
    #[livre(rename = "ID")]
    pub id: Option<Id>,
}


/// A special type to parse previous trailers. Since we only need to get the `prev` key in previous
/// trailers, we create here a model that only recognizes that.
#[derive(Debug, Clone, PartialEq, FromRawDict)]
pub(crate) struct PrevTrailer {
    /// The only valuable key in previous trailers, since it points to the previous
    /// xref-table/trailer tuple.
    pub prev: Option<usize>
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
            id: Some([[0x2b, 0x55], [0x0a, 0x12]].into()),
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
        PrevTrailer{
            prev: Some(116),
        }
    )]
    #[case(
        indoc!{b"
            <</Size 92813
            /Root 90794 0 R
            /Info 90792 0 R
            /ID[<2B55><0a12>]
            /Type/XRef
            /W[ 1 3 0]
            /Index[ 1 1 7 1 14 1 16 1 91807 1006]
            /Length 1>>\
        "},
        PrevTrailer{
            prev: None,
        }
    )]
    // "impossible" cases.. still interesting for testing, maybe?
    #[case(
        b"<</Prev 116>>",
        PrevTrailer{
            prev: Some(116),
        }
    )]
    #[case(
        b"<<>>",
        PrevTrailer{
            prev: None,
        }
    )]
    fn prev_trailer_extraction<'de, T>(#[case] input: &'de [u8], #[case] expected: T)
    where
        T: Extract<'de> + Debug + PartialEq,
    {
        let result = extract(&mut input.as_ref()).unwrap();
        // let (_, result) = extract_deserialize(input).unwrap();
        assert_eq!(expected, result);
    }
}
