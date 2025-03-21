//! Filters for PDF stream objects.

use enum_dispatch::enum_dispatch;
use std::io::Read;
use winnow::{
    combinator::fail,
    error::{ContextError, ErrMode, StrContext},
    BStr, PResult, Parser,
};

use flate2::read::ZlibDecoder;

use crate::{
    extraction::{extract, Extract, Name},
    follow_refs::{Build, Builder},
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
    fn decode(&self, bytes: &[u8]) -> Result<Vec<u8>, ()>;
}

impl Extract<'_> for Filter {
    fn extract(input: &mut &BStr) -> PResult<Self> {
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
    fn build<B>(input: &mut &BStr, _builder: &B) -> PResult<Self>
    where
        B: Builder,
    {
        extract(input)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlateDecode;

impl Filtering for FlateDecode {
    fn decode(&self, bytes: &[u8]) -> Result<Vec<u8>, ()> {
        let mut d = ZlibDecoder::new(bytes);
        let mut buf = Vec::new();
        d.read_to_end(&mut buf).map_err(|_| ())?;
        Ok(buf)
    }
}

impl<T> Filtering for Vec<T>
where
    T: Filtering,
{
    fn decode(&self, bytes: &[u8]) -> Result<Vec<u8>, ()> {
        let mut result = bytes.to_vec();
        for decoder in self {
            result = decoder.decode(&result)?;
        }
        Ok(result)
    }
}
