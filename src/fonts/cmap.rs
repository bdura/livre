use std::collections::BTreeMap;

use livre_derive::{BuildFromRawDict, FromRawDict};
use winnow::combinator::{empty, fail};
use winnow::dispatch;
use winnow::token::any;
use winnow::{BStr, PResult, Parser};

use crate::extraction::{extract, Extract, Stream, Todo};
use crate::follow_refs::{Build, Builder};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub enum WritingMode {
    #[default]
    Horizontal,
    Vertical,
}

impl Extract<'_> for WritingMode {
    fn extract(input: &mut &BStr) -> PResult<Self> {
        dispatch! {
            any;
            b'0' => empty.value(Self::Horizontal),
            b'1' => empty.value(Self::Vertical),
            _ => fail,
        }
        .parse_next(input)
    }
}

impl Build for WritingMode {
    fn build<B>(input: &mut &BStr, _: &B) -> PResult<Self>
    where
        B: crate::follow_refs::Builder,
    {
        extract(input)
    }
}

#[derive(Debug, Clone, PartialEq, Copy, FromRawDict, BuildFromRawDict)]
pub struct CMapConfig {
    #[livre(default)]
    w_mode: WritingMode,
    use_c_map: Option<Todo>,
}

/// Mapping from character code to character selector. In a CIDFont, the selector
/// is a CID (character identifier).
///
/// There is also a special type of `CMap`, the [`ToUnicode`] Cmap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CMap<I> {
    pub config: CMapConfig,
    pub inner: I,
}

impl<'de, I> Extract<'de> for CMap<I>
where
    I: for<'a> Extract<'a>,
{
    fn extract(input: &mut &'de BStr) -> PResult<Self> {
        let Stream {
            structured: config,
            content,
        } = Stream::extract(input)?;

        let inner = extract(&mut content.as_slice().as_ref())?;

        Ok(Self { config, inner })
    }
}

impl<I> Build for CMap<I>
where
    I: for<'a> Extract<'a>,
{
    fn build<B>(input: &mut &BStr, builder: &B) -> PResult<Self>
    where
        B: Builder,
    {
        let Stream {
            structured: config,
            content,
        } = Stream::build(input, builder)?;

        let inner = extract(&mut content.as_slice().as_ref())?;

        Ok(Self { config, inner })
    }
}

/// Special kind of CMap that links character codes to their unicode representation.
#[derive(Debug, Clone, PartialEq)]
pub struct ToUnicode(pub BTreeMap<u16, Vec<u16>>);
