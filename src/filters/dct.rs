use super::error::Result;
use super::Filtering;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DCTDecode;

impl Filtering for DCTDecode {
    fn decode(&self, _: &[u8]) -> Result<Vec<u8>> {
        todo!()
    }
}