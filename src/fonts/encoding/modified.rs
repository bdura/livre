use livre_derive::FromRawDict;
use winnow::{BStr, PResult, Parser};

use crate::extraction::{extract, Extract, Name, Repeated};

use super::{BuiltInEncoding, CharacterSet, Encoding, Glyph};

/// A more complex encoding that can be present in a PDF document.
///
/// Livre's `ModifiedEncoding` is the distilled version of an encoding dictionary,
/// with the base encoding and the differences compiled into a single array.
#[derive(Debug)]
pub struct ModifiedEncoding(pub CharacterSet);

impl Encoding for ModifiedEncoding {
    fn to_char(&self, code: u8) -> u16 {
        self.0[code as usize].expect("character code should be present")
    }

    fn character_set(self) -> CharacterSet {
        self.0
    }
}

impl Extract<'_> for ModifiedEncoding {
    fn extract(input: &mut &BStr) -> PResult<Self> {
        let EncodingDictionary {
            base_encoding,
            differences,
        } = extract(input)?;

        let mut character_set = base_encoding.character_set();

        for difference in differences {
            difference.modify_set(&mut character_set);
        }

        Ok(Self(character_set))
    }
}

/// Helper type used during the extraction of a [`ModifiedEncoding`].
///
/// Represents the encoding dictionary.
#[derive(FromRawDict)]
struct EncodingDictionary {
    #[livre(default)]
    base_encoding: BuiltInEncoding,
    differences: Vec<EncodingDifference>,
}

/// Helper for the extraction of a [`ModifiedEncoding`].
///
/// Represents a single encoding difference, in the format:
///
/// ```raw
/// code name_1 name_2 name_3 ... name_n
/// ```
struct EncodingDifference(usize, Vec<Option<u16>>);

impl EncodingDifference {
    /// Modify the basis character set inplace.
    fn modify_set(self, character_set: &mut [Option<u16>]) {
        for (i, e) in (self.0..).zip(self.1) {
            character_set[i] = e;
        }
    }
}

impl Extract<'_> for EncodingDifference {
    fn extract(input: &mut &BStr) -> PResult<Self> {
        let (code, Repeated::<DifferenceElement>(differences)) = extract(input)?;
        let differences = differences
            .into_iter()
            .map(|DifferenceElement(inner)| inner)
            .collect();
        Ok(Self(code, differences))
    }
}

/// Helper type for the extraction of a [`ModifiedEncoding`].
///
/// Extracts a single glyph name. Using this helper type avoids allocating a vector
/// of [`Name`]s by performing the conversion on the fly.
struct DifferenceElement(Option<u16>);

impl Extract<'_> for DifferenceElement {
    fn extract(input: &mut &BStr) -> PResult<Self> {
        Glyph.map(Self).parse_next(input)
    }
}
