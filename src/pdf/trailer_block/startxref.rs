use std::fmt::Debug;

use winnow::{ascii::multispace1, token::take_until, BStr, PResult, Parser};

use crate::extraction::{extract, Extract};

/// Extractor type for the `startxref` tag in a PDF document.
#[derive(Debug, Clone, Copy)]
pub struct StartXRef(pub usize);

impl Extract<'_> for StartXRef {
    fn extract(input: &mut &BStr) -> PResult<Self> {
        let (_, _, value) = (b"startxref", multispace1, extract).parse_next(input)?;
        Ok(Self(value))
    }
}

impl StartXRef {
    pub fn find(input: &BStr) -> PResult<Self> {
        const MAXIMUM_XREF_LEN: usize = 30;

        // Rush to the end
        let i = &mut &input[(input.len().saturating_sub(MAXIMUM_XREF_LEN))..];
        // Look for the tag
        take_until(0.., b"startxref".as_slice()).parse_next(i)?;
        // Extract tag + value
        Self::extract(i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use indoc::indoc;

    #[test]
    fn startxref() {
        let input = indoc! {b"
            test
            test
            test
            test
            test
            test
            test
            test
            test
            test
            startxref
            7
        "}
        .as_slice();

        assert_eq!(input.len(), 62);

        let StartXRef(value) = StartXRef::find(input.as_ref()).unwrap();

        assert_eq!(input.len(), 62);
        assert_eq!(value, 7);
    }
}
