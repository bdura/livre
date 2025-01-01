use crate::extraction::{extract, Extract, HexadecimalString};

/// PDF ID.
///
/// From the specification:
///
/// > PDF file identifiers shall be defined by the ID entry in a PDF file’s trailer dictionary
/// > (see 7.5.5, "File trailer"). The value of this entry shall be an array of two byte strings.
/// > The first byte string shall be a permanent identifier based on the contents of the PDF file
/// > at the time it was originally created and shall not change when the PDF file is updated.
/// > The second byte string shall be a changing identifier based on the PDF file’s contents at
/// > the time it was last updated (see 7.5.6, "Incremental updates"). When a PDF file is first
/// > written, both identifiers shall be set to the same value. If the first identifier in the
/// > reference matches the first identifier in the referenced file’s ID entry, and the last
/// > identifier in the reference matches the last identifier in the referenced file’s ID entry,
/// > it is very likely that the correct and unchanged PDF file has been found. If only the first
/// > identifier matches, a different version of the correct PDF file has been found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Id {
    creation: Vec<u8>,
    modification: Vec<u8>,
}

impl Id {
    /// Check whether the document has been modified.
    ///
    /// Note that using the [`Id`] for that is not sufficient, since it relies on a hashing
    /// algorithm - what is more, that algorithm is chosen by the PDF creation software and even
    /// the hash length is unspecified.
    ///
    /// Hence, although you can be certain that the document was in fact modified if the return
    /// value is `true`, the converse does not hold. In this context, `false` means: "probably
    /// not".
    pub fn was_modified(&self) -> bool {
        self.creation != self.modification
    }
}

impl Extract<'_> for Id {
    fn extract(input: &mut &'_ winnow::BStr) -> winnow::PResult<Self> {
        let [HexadecimalString(creation), HexadecimalString(modification)] = extract(input)?;
        Ok(Self {
            creation,
            modification,
        })
    }
}

impl<T> From<[T; 2]> for Id
where
    T: Into<Vec<u8>>,
{
    fn from([c, m]: [T; 2]) -> Self {
        let creation = c.into();
        let modification = m.into();
        Self {
            creation,
            modification,
        }
    }
}

#[cfg(test)]
mod tests {

    use std::fmt::Debug;

    use rstest::rstest;

    use crate::extraction::{extract, Extract};

    use super::*;

    #[rstest]
    #[case(b"[<2B55><0a12>]", Id{creation: vec![0x2b, 0x55], modification: vec![0x0a, 0x12]})]
    fn extraction<'de, T>(#[case] input: &'de [u8], #[case] expected: T)
    where
        T: Extract<'de> + Debug + PartialEq,
    {
        let result = extract(&mut input.as_ref()).unwrap();
        assert_eq!(expected, result);
    }
}
