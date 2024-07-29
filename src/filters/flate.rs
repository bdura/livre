use std::io::Read;

use super::error::Result;
use flate2::read::ZlibDecoder;

use super::Filtering;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlateDecode;

impl Filtering for FlateDecode {
    fn decode(&self, bytes: &[u8]) -> Result<Vec<u8>> {
        let mut d = ZlibDecoder::new(bytes);
        let mut buf = Vec::new();
        d.read_to_end(&mut buf)?;
        Ok(buf)
    }
}
