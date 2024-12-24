mod plain;
mod stream;

use std::fmt::Debug;

use winnow::{ascii::multispace1, combinator::alt, token::take_until, BStr, PResult, Parser};

use crate::extraction::{extract, Extract, ReferenceId};

use super::Trailer;

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
        let mut i = &input[(input.len().saturating_sub(MAXIMUM_XREF_LEN))..];
        // Look for the tag
        take_until(0.., b"startxref".as_slice()).parse_next(&mut i)?;
        // Extract tag + value
        Self::extract(&mut i)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum RefLocation {
    Plain(usize),
    Compressed(usize),
}

impl RefLocation {
    pub fn from_offset_and_flag(offset: usize, compressed: bool) -> Self {
        if compressed {
            Self::Compressed(offset)
        } else {
            Self::Plain(offset)
        }
    }
}

pub fn extract_xref(input: &mut &BStr) -> PResult<(Trailer, Vec<(ReferenceId, RefLocation)>)> {
    alt((plain::xref, stream::xref)).parse_next(input)
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
