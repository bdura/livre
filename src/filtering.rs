//! Filters for PDF stream objects.
use enum_dispatch::enum_dispatch;
use std::io::Read;
use winnow::{
    error::{ContextError, ErrMode},
    BStr, PResult,
};

use flate2::read::ZlibDecoder;

use crate::extraction::{extract, Extract, Name};

#[enum_dispatch]
pub trait Filtering {
    fn decode(&self, bytes: &[u8]) -> Result<Vec<u8>, ()>;
}

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

impl Extract<'_> for Filter {
    fn extract(input: &mut &BStr) -> PResult<Self> {
        let Name(value) = extract(input)?;

        match value.as_slice() {
            b"FlateDecode" => Ok(Self::FlateDecode(FlateDecode)),
            b"ASCII85Decode" | b"ASCIIHexDecode" | b"LZWDecode" | b"RunLengthDecode"
            | b"CCITTFaxDecode" | b"JBIG2Decode" | b"DCTDecode" | b"JPXDecode" | b"Crypt" => {
                todo!(
                    "{} filter is not handled by Livre yet. Consider opening an issue, or better yet, submitting a PR!",
                    // SAFETY: we just matched `value` against an UTF8-encoded string.
                    unsafe { std::str::from_utf8_unchecked(value.as_slice()) }
                )
            }
            _ => Err(ErrMode::Backtrack(ContextError::new())),
        }
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
