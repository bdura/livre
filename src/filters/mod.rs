use enum_dispatch::enum_dispatch;

use crate::error::Result;

mod flate;
pub use flate::FlateDecode;

mod dct;
pub use dct::DCTDecode;

#[enum_dispatch(Filter)]
pub trait Filtering {
    fn decode(&self, bytes: &[u8]) -> Result<Vec<u8>>;
}

#[derive(Debug, Clone, PartialEq)]
#[enum_dispatch]
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
