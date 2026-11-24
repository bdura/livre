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

pub(crate) use predictor::Predictor;

use crate::{
    extraction::{extract, Extract, FromRawDict, Name},
    follow_refs::{build_via_extract, BuildFromRawDict},
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
    /// Decode `bytes` with this filter and the parameters it carries.
    fn decode(&self, bytes: &[u8]) -> Result<Vec<u8>, ()>;
}

impl Filter {
    /// Attach this filter's `/DecodeParms` entry.
    ///
    /// Returns `Err(())` when the parameters are invalid for this filter.
    pub(crate) fn with_params(self, params: FlateParams) -> Result<Self, ()> {
        match self {
            Self::FlateDecode(_) => Ok(Self::FlateDecode(FlateDecode {
                predictor: Predictor::try_from(params)?,
            })),
        }
    }
}

/// One `/DecodeParms` dictionary, as read by `FlateDecode` (ISO 32000-2, Table 8).
///
/// Absent keys take the spec defaults. Values are not validated here: a
/// stream whose predictor is 1 decodes whatever the other entries say.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRawDict, BuildFromRawDict)]
pub struct FlateParams {
    /// `1` for none, `2` for TIFF Predictor 2, `10..=15` for PNG predictors.
    #[livre(default = 1)]
    pub predictor: u8,
    /// Interleaved colour components per sample.
    #[livre(default = 1)]
    pub colors: usize,
    /// Bits per colour component: `1`, `2`, `4`, `8` or `16`.
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
            b"FlateDecode" => Ok(Self::FlateDecode(FlateDecode::default())),
            b"ASCII85Decode" | b"ASCIIHexDecode" | b"LZWDecode" | b"RunLengthDecode"
            | b"CCITTFaxDecode" | b"JBIG2Decode" | b"DCTDecode" | b"JPXDecode" | b"Crypt" => fail
                .context(StrContext::Label("unsupported filter"))
                .parse_next(input),
            _ => Err(ErrMode::Backtrack(ContextError::new())),
        }
    }
}

build_via_extract!(Filter);

/// Zlib inflation, then the `/Predictor` step if the stream has one.
///
/// The default is the filter with no parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FlateDecode {
    predictor: Predictor,
}

impl Filtering for FlateDecode {
    fn decode(&self, bytes: &[u8]) -> Result<Vec<u8>, ()> {
        let mut d = ZlibDecoder::new(bytes);
        let mut buf = Vec::new();
        d.read_to_end(&mut buf).map_err(|_| ())?;

        self.predictor.undo(buf)
    }
}
