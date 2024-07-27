use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
enum _MaybeArray<T> {
    Array(Vec<T>),
    Single(T),
}

impl<T> From<_MaybeArray<T>> for Vec<T> {
    fn from(value: _MaybeArray<T>) -> Self {
        match value {
            _MaybeArray::Array(array) => array,
            _MaybeArray::Single(value) => vec![value],
        }
    }
}

impl<T> From<_MaybeArray<T>> for MaybeArray<T> {
    fn from(value: _MaybeArray<T>) -> Self {
        Self(value.into())
    }
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(from = "_MaybeArray<T>")]
pub struct MaybeArray<T>(pub Vec<T>);

impl<T> Default for MaybeArray<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use rstest::rstest;

    use super::*;
    use crate::from_bytes;

    #[rstest]
    #[case(b"[12 13]", _MaybeArray::Array(vec![12, 13]))]
    #[case(b"12", _MaybeArray::Single(12))]
    fn maybe_array<T>(#[case] input: &[u8], #[case] expected: _MaybeArray<T>)
    where
        T: for<'de> Deserialize<'de> + PartialEq + Debug,
    {
        assert_eq!(expected, from_bytes(input).unwrap());
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct Test<T> {
        #[serde(default)]
        vec: MaybeArray<T>,
    }

    #[rstest]
    #[case(b"<</vec[12 13]>>", vec![12, 13])]
    #[case(b"<</vec[42]>>", vec![42])]
    #[case(b"<</vec 42>>", vec![42])]
    #[case(b"<</vec [true false]>>", vec![true, false])]
    #[case(b"<<>>", Vec::<bool>::new())]
    fn deserialize_struct_maybe_array<'de, T>(#[case] input: &'de [u8], #[case] expected: Vec<T>)
    where
        T: Deserialize<'de> + PartialEq + Debug + Default,
    {
        let Test {
            vec: MaybeArray(array),
        } = from_bytes(input).unwrap();

        assert_eq!(expected, array);
    }
}
