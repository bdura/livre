use livre_derive::{BuildFromRawDict, FromRawDict};
use winnow::{BStr, PResult, Parser};

use crate::{
    extraction::{extract, Extract, Repeated},
    follow_refs::{Build, Builder},
};

use super::{BuiltInEncoding, Decode, Glyph};

/// A more complex encoding that can be present in a PDF document.
///
/// Livre's `ModifiedEncoding` is the distilled version of an encoding dictionary,
/// with the base encoding and the differences compiled into a single array.
#[derive(Debug, Clone, PartialEq)]
pub struct ModifiedEncoding(pub Vec<Option<u16>>);

impl Decode for ModifiedEncoding {
    fn decode(&self, code: u8) -> u16 {
        self.0[code as usize].expect("character code should be present")
    }

    fn character_set(self) -> Vec<Option<u16>> {
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

impl Build for ModifiedEncoding {
    fn build<B>(input: &mut &BStr, builder: &B) -> PResult<Self>
    where
        B: Builder,
    {
        let EncodingDictionary {
            base_encoding,
            differences,
        } = builder.build(input)?;

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
#[derive(FromRawDict, BuildFromRawDict)]
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

impl Build for EncodingDifference {
    // NOTE: we are making the assumption that the differences come as a block.
    fn build<B>(input: &mut &BStr, _: &B) -> PResult<Self>
    where
        B: Builder,
    {
        extract(input)
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

#[cfg(test)]
mod tests {
    use super::*;

    use indoc::indoc;

    #[test]
    fn modified_encoding() {
        let input: &[u8] = indoc! {b"
            <</Type /Encoding
                /Differences
                    [39 /quotesingle
                    96 /grave

                    128 /Adieresis /Aring /Ccedilla /Eacute /Ntilde /Odieresis /Udieresis
                        /aacute /agrave /acircumflex /adieresis /atilde /aring /ccedilla
                        /eacute /egrave /ecircumflex /edieresis /iacute /igrave /icircumflex
                        /idieresis /ntilde /oacute /ograve /ocircumflex /odieresis /otilde
                        /uacute /ugrave /ucircumflex /udieresis /dagger /degree /cent
                        /sterling /section /bullet /paragraph /germandbls /registered
                        /copyright /trademark /acute /dieresis

                    174 /AE /Oslash
                    177 /plusminus
                    180 /yen /mu
                    187 /ordfeminine /ordmasculine
                    190 /ae /oslash /questiondown /exclamdown /logicalnot
                    196 /florin
                    199 /guillemotleft /guillemotright /ellipsis
                    203 /Agrave /Atilde /Otilde /OE /oe /endash /emdash /quotedblleft
                        /quotedblright /quoteleft /quoteright /divide

                    216 /ydieresis /Ydieresis /fraction /currency /guilsinglleft /guilsinglright
                        /fi /fl /daggerdbl /periodcentered /quotesinglbase /quotedblbase
                        /perthousand /Acircumflex /Ecircumflex /Aacute /Edieresis /Egrave
                        /Iacute /Icircumflex /Idieresis /Igrave /Oacute /Ocircumflex

                    241 /Ograve /Uacute /Ucircumflex /Ugrave /dotlessi /circumflex /tilde
                        /macron /breve /dotaccent /ring /cedilla /hungarumlaut /ogonek /caron
                ]
            >>
        "};

        let encoding: ModifiedEncoding = extract(&mut input.as_ref()).unwrap();

        assert_eq!(encoding.decode(128), Glyph::Adieresis);
        assert_eq!(encoding.decode(129), Glyph::Aring);
        assert_eq!(encoding.decode(136), Glyph::agrave);
    }
}
