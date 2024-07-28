//! Filters for PDF stream objects.

use std::borrow::Cow;

use enum_dispatch::enum_dispatch;

mod error;
pub use error::Result;

mod filters;
use filters::{DCTDecode, FlateDecode};
use serde::Deserialize;

#[enum_dispatch]
pub trait Filtering {
    fn decode(&self, bytes: &[u8]) -> Result<Vec<u8>>;
}

#[derive(Deserialize)]
enum FilterName {
    FlateDecode,
    DCTDecode,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(from = "FilterName")]
#[enum_dispatch(Filtering)]
pub enum Filter {
    FlateDecode(FlateDecode),
    DCTDecode(DCTDecode),
}

impl<T: Filtering> Filtering for Vec<T> {
    fn decode(&self, bytes: &[u8]) -> Result<Vec<u8>> {
        let mut cow = Cow::Borrowed(bytes);
        for decoder in self {
            let result = decoder.decode(cow.as_ref())?;
            cow = Cow::Owned(result);
        }
        Ok(cow.into_owned())
    }
}

impl From<FilterName> for Filter {
    fn from(value: FilterName) -> Self {
        match value {
            FilterName::FlateDecode => Self::FlateDecode(FlateDecode),
            FilterName::DCTDecode => Self::DCTDecode(DCTDecode),
        }
    }
}

#[cfg(test)]
mod tests {
    use livre_serde::from_bytes;

    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(b"/FlateDecode", FlateDecode.into())]
    #[case(b"  /FlateDecode", FlateDecode.into())]
    #[case(b"/DCTDecode", DCTDecode.into())]
    #[should_panic]
    #[case(b"/NonExistent", DCTDecode.into())]
    fn filter_deserialize(#[case] input: &[u8], #[case] expected: Filter) {
        assert_eq!(expected, from_bytes(input).unwrap());
    }
}
