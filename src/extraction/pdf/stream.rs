use winnow::{
    combinator::{alt, trace},
    error::{ContextError, ErrMode},
    BStr, PResult, Parser,
};

use crate::{extraction::FromRawDict, filtering::Filter, Extract};

use super::RawDict;

#[derive(Debug)]
struct MaybeArray<T>(pub Vec<T>);

impl<T> From<MaybeArray<T>> for Vec<T> {
    fn from(value: MaybeArray<T>) -> Self {
        value.0
    }
}

impl<'de, T> Extract<'de> for MaybeArray<T>
where
    T: Extract<'de>,
{
    fn extract(input: &mut &'de BStr) -> PResult<Self> {
        trace(
            "livre-maybe-array",
            alt((T::extract.map(|t| vec![t]), Vec::<T>::extract)).map(Self),
        )
        .parse_next(input)
    }
}

#[derive(Debug, PartialEq, Eq)]
struct StreamDict {
    length: usize,
    filter: Vec<Filter>,
}

impl FromRawDict<'_> for StreamDict {
    fn from_dict(dict: &mut RawDict) -> PResult<Self> {
        let length: usize = dict
            .pop_and_extract(&"Length".into())
            .ok_or(ErrMode::Backtrack(ContextError::new()))??;

        let filter = if let Some(filter) = dict.pop(&"Filter".into()) {
            let filter: MaybeArray<Filter> = filter.extract()?;
            filter.into()
        } else {
            Vec::new()
        };

        let result = Self { length, filter };
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::extraction::extract;

    use super::*;

    #[rstest()]
    #[case(b"<</Length 2/SomeOtherKey/Test>>", StreamDict{length: 2, filter: vec![]})]
    #[case(b"<</Length 42>>", StreamDict{length: 42, filter: vec![]})]
    #[case(b"<<  /SomeRandomKey (some text...)/Length 42>>", StreamDict{length: 42, filter: vec![]})]
    fn stream_dict(#[case] input: &[u8], #[case] expected: StreamDict) {
        let result = extract(&mut input.as_ref()).unwrap();
        assert_eq!(expected, result);
    }
}
