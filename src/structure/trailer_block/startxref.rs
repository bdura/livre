use std::fmt::Debug;

use winnow::{
    ascii::multispace1,
    error::{ContextError, ErrMode},
    BStr, ModalResult, Parser,
};

use crate::extraction::{extract, Extract};

/// Extractor type for the `startxref` tag in a PDF document.
#[derive(Debug, Clone, Copy)]
pub struct StartXRef(pub usize);

impl Extract<'_> for StartXRef {
    fn extract(input: &mut &BStr) -> ModalResult<Self> {
        let (_, _, value) = (b"startxref", multispace1, extract).parse_next(input)?;
        Ok(Self(value))
    }
}

impl StartXRef {
    pub fn find(input: &BStr) -> ModalResult<Self> {
        // The PDF spec requires `%%EOF` to appear within the last 1024 bytes,
        // so `startxref` is always within this window.
        const SEARCH_WINDOW: usize = 1024;

        const TAG: &[u8] = b"startxref";

        let window = &input[input.len().saturating_sub(SEARCH_WINDOW)..];

        // Scan backward to find the *last* `startxref` — incremental PDF updates
        // append new xref tables at the end, so the last occurrence is the one we want.
        let pos = window
            .windows(TAG.len())
            .rposition(|w| w == TAG)
            .ok_or(ErrMode::Cut(ContextError::new()))?;

        Self::extract(&mut window[pos..].into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use indoc::indoc;

    #[test]
    fn startxref() {
        let input = indoc! {b"
            startxref
            7
        "}
        .as_slice();

        let StartXRef(value) = StartXRef::find(input.as_ref()).unwrap();
        assert_eq!(value, 7);
    }

    /// When a PDF has been incrementally updated, multiple `startxref` markers
    /// exist in the file. `find` must return the last one (the most recent xref).
    #[test]
    fn startxref_picks_last_occurrence() {
        let input = indoc! {b"
            startxref
            7
            %%EOF
            startxref
            42
            %%EOF
        "}
        .as_slice();

        let StartXRef(value) = StartXRef::find(input.as_ref()).unwrap();
        assert_eq!(value, 42);
    }
}
