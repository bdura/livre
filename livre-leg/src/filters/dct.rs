use crate::error::Result;

use super::Filtering;

#[derive(Debug, Clone, PartialEq)]
pub struct DCTDecode;

impl Filtering for DCTDecode {
    fn decode(&self, _: &[u8]) -> Result<Vec<u8>> {
        todo!()
    }
}
