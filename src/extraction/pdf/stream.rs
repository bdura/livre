use winnow::{
    error::{ContextError, ErrMode},
    PResult,
};

use crate::extraction::FromRawDict;

use super::RawDict;

#[derive(Debug, PartialEq, Eq)]
struct StreamDict {
    length: usize,
}

impl FromRawDict<'_> for StreamDict {
    fn from_dict(dict: &mut RawDict) -> PResult<Self> {
        let length: usize = dict
            .pop_and_extract(&"Length".into())
            .ok_or(ErrMode::Backtrack(ContextError::new()))??;

        let result = Self { length };
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::extraction::extract;

    use super::*;

    #[rstest()]
    #[case(b"<</Length 2/SomeOtherKey/Test>>", StreamDict{length: 2})]
    #[case(b"<</Length 42>>", StreamDict{length: 42})]
    #[case(b"<<  /SomeRandomKey (some text...)/Length 42>>", StreamDict{length: 42})]
    fn stream_dict(#[case] input: &[u8], #[case] expected: StreamDict) {
        let result = extract(&mut input.as_ref()).unwrap();
        assert_eq!(expected, result);
    }
}
