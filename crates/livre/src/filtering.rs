//! Filters for PDF stream objects.

use enum_dispatch::enum_dispatch;
use std::io::Read;
use winnow::{
    combinator::fail,
    error::{ContextError, ErrMode, StrContext},
    BStr, ModalResult, Parser,
};

use flate2::read::ZlibDecoder;

mod predictor;

use crate::{
    extraction::{extract, Extract, FromRawDict, Name},
    follow_refs::{Build, BuildFromRawDict, Builder},
};

/// Main filter objects, that represents any kind of PDF filter.
///
/// This is the only type you should need for PDF parsing.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[enum_dispatch(Filtering)]
pub enum Filter {
    FlateDecode,
    // ASCII85Decode,
    // ASCIIHexDecode,
    // LZWDecode,
    // RunLengthDecode,
    // CCITTFaxDecode,
    // JBIG2Decode,
    // DCTDecode,
    // JPXDecode,
    // Crypt,
}

/// Filters are used in [PDF streams](crate::extraction::Stream) to define and configure
/// processing steps such as the compression algorithm used to generate the stream content
/// (if any), the encryption scheme, etc.
#[enum_dispatch]
pub trait Filtering {
    /// Decode `bytes`, using this filter's entry in the stream's `/DecodeParms`
    /// if there is one.
    fn decode(&self, bytes: &[u8], params: Option<&FlateParams>) -> Result<Vec<u8>, ()>;
}

/// One `/DecodeParms` dictionary, as read by `FlateDecode` (ISO 32000-2, Table 8).
///
/// Absent keys take the spec defaults. Values are not validated here: a
/// stream whose predictor is 1 decodes whatever the other entries say.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRawDict, BuildFromRawDict)]
pub struct FlateParams {
    #[livre(default = 1)]
    pub predictor: u8,
    /// Interleaved colour components per sample.
    #[livre(default = 1)]
    pub colors: usize,
    #[livre(default = 8)]
    pub bits_per_component: usize,
    /// Samples per row.
    #[livre(default = 1)]
    pub columns: usize,
}

impl Extract<'_> for Filter {
    fn extract(input: &mut &BStr) -> ModalResult<Self> {
        let Name(value) = extract(input)?;

        match value.as_slice() {
            b"FlateDecode" => Ok(Self::FlateDecode(FlateDecode)),
            b"ASCII85Decode" | b"ASCIIHexDecode" | b"LZWDecode" | b"RunLengthDecode"
            | b"CCITTFaxDecode" | b"JBIG2Decode" | b"DCTDecode" | b"JPXDecode" | b"Crypt" => fail
                .context(StrContext::Label("unsupported filter"))
                .parse_next(input),
            _ => Err(ErrMode::Backtrack(ContextError::new())),
        }
    }
}

impl Build for Filter {
    fn build<B>(input: &mut &BStr, _builder: &B) -> ModalResult<Self>
    where
        B: Builder,
    {
        extract(input)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlateDecode;

impl Filtering for FlateDecode {
    fn decode(&self, bytes: &[u8], params: Option<&FlateParams>) -> Result<Vec<u8>, ()> {
        let mut d = ZlibDecoder::new(bytes);
        let mut buf = Vec::new();
        d.read_to_end(&mut buf).map_err(|_| ())?;

        match params {
            Some(p) if p.predictor > 1 => {
                predictor::undo(buf, p.predictor, p.colors, p.bits_per_component, p.columns)
            }
            _ => Ok(buf),
        }
    }
}
