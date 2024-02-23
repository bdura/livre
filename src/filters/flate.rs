use std::io::Read;

use flate2::read::ZlibDecoder;

use crate::error::Result;

use super::Filtering;

#[derive(Debug, Clone, PartialEq)]
pub struct FlateDecode;

impl Filtering for FlateDecode {
    fn decode(&self, bytes: &[u8]) -> Result<Vec<u8>> {
        let mut d = ZlibDecoder::new(bytes);
        let mut buf = Vec::new();
        d.read_to_end(&mut buf)?;
        Ok(buf)
    }
}

// pub enum FlateDecodeConfig {
//     Normal,
//     Predictor(PredictorConfig),
// }

// pub struct PredictorConfig {
//     colors: u8,
//     bits_per_components: Bpc,
//     columns: u8,
// }

// pub enum Bpc {
//     Bpc1,
//     Bpc2,
//     Bpc4,
//     Bpc8,
//     Bpc16,
// }
