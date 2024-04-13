//! Filters for PDF stream objects.

use enum_dispatch::enum_dispatch;

mod error;
pub use error::Result;

mod filters;
pub use filters::{DCTDecode, FlateDecode};
use livre_extraction::{Extract, IResult, Name};

#[enum_dispatch]
pub trait Filtering {
    fn decode(&self, bytes: &[u8]) -> Result<Vec<u8>>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[enum_dispatch(Filtering)]
pub enum Filter {
    FlateDecode(FlateDecode),
    DCTDecode(DCTDecode),
}

impl Filter {
    pub fn from_name(name: &str) -> Self {
        match name {
            "ASCIIHexDecode" => todo!(),
            "ASCII85Decode" => todo!(),
            "LZWDecode" => todo!(),
            "FlateDecode" => FlateDecode.into(),
            "RunLengthDecode" => todo!(),
            "CCITTFaxDecode" => todo!(),
            "JBIG2Decode" => todo!(),
            "DCTDecode" => DCTDecode.into(),
            "JPXDecode" => todo!(),
            "Crypt" => todo!(),
            _ => unreachable!("Unknown filter."),
        }
    }
}

impl<'input> Extract<'input> for Filter {
    fn extract(input: &'input [u8]) -> IResult<&'input [u8], Self> {
        let (input, Name(name)) = Name::extract(input)?;
        let filter = Filter::from_name(&name);
        Ok((input, filter))
    }
}
