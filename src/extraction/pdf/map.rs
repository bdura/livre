use std::collections::HashMap;

use winnow::{
    ascii::multispace0,
    combinator::{fail, iterator, peek, separated_pair, terminated, trace},
    dispatch,
    token::any,
    BStr, PResult, Parser,
};

use crate::{
    extraction::{
        extract,
        primitives::recognize_number,
        utilities::{Angles, Brackets, DoubleAngles, Parentheses},
    },
    Extract,
};

use super::name::Name;

/// In PDFs, dictionary keys are [`Name`]s.
type Map<T> = HashMap<Name, T>;

impl<'de, T> Extract<'de> for Map<T>
where
    T: Extract<'de>,
{
    fn extract(input: &mut &'de BStr) -> winnow::PResult<Self> {
        trace("livre-map", |i: &mut &'de BStr| {
            let DoubleAngles(mut inside) = extract(i)?;

            // Remove leading spaces
            multispace0(&mut inside)?;

            // `parse_key_value` includes trailing spaces
            let mut it = iterator(inside, parse_key_value);

            let map = it.collect();
            let (i, _) = it.finish()?;

            // TODO: remove this panic... Useful for now, it lets us know if
            // something went wrong.
            assert!(
                i.is_empty(),
                "Input not empty after parsing a dictionary: {:?}",
                i
            );

            Ok(map)
        })
        .parse_next(input)
    }

    fn recognize(input: &mut &'de winnow::BStr) -> winnow::PResult<&'de [u8]> {
        DoubleAngles::extract.take().parse_next(input)
    }
}

/// Parse a single key-value pair. Consumes trailing whitespace if there is any.
fn parse_key_value<'de, T>(input: &mut &'de BStr) -> PResult<(Name, T)>
where
    T: Extract<'de>,
{
    terminated(
        separated_pair(Name::extract, multispace0, T::extract),
        multispace0,
    )
    .parse_next(input)
}

/// A container for **any** PDF type that may appear, to be parsed by a
/// dedicated parser. After the fact.
///
/// Idea: replace this logic with an iterator-based extraction, where the
/// key is directly extracted using the dedicated parser.
/// This is likely an involved endeavour. The trickiest part is for enums:
/// how do you know which type this is ahead of time?
///
/// This is the issue with serde, which starts mapping everything to its
/// internal data model, which works with well-defined formats but not
/// with PDFs...
///
/// I see two solutions:
///
/// 1. Parse as `RawValue`, handle it once we know which type this is
/// 2. Iterate once to get the type, then iterate knowing the type.
#[derive(Debug, PartialEq)]
pub struct RawValue<'de>(pub &'de BStr);

impl<'de> RawValue<'de> {
    pub fn extract<T>(mut self) -> PResult<T>
    where
        T: Extract<'de>,
    {
        extract(&mut self.0)
    }
}

impl<'de> From<&'de [u8]> for RawValue<'de> {
    fn from(value: &'de [u8]) -> Self {
        Self(value.into())
    }
}

impl<'de> Extract<'de> for RawValue<'de> {
    fn extract(input: &mut &'de BStr) -> PResult<Self> {
        dispatch! {peek(any);
            b'/' => Name::recognize,
            b'[' => Brackets::recognize,
            b'(' => Parentheses::recognize,
            b'<' => Angles::recognize,
            b'+' | b'-' | b'.' | b'0'..=b'9' => recognize_number,
            b't' | b'f' => bool::recognize,
            b'n' => b"null",
            // FIXME: this is in fact trickier... It could be a tuple, which is
            // ill-defined.
            // In practice, it's probably ok to assume that no tuple includes a
            // PDF name, which makes things easier.
            _ => fail,
        }
        .map(Self::from)
        .parse_next(input)
    }
}

/// A dictionary instance that keeps values in their raw form.
#[derive(Debug, PartialEq)]
pub struct RawDict<'de>(Map<RawValue<'de>>);

impl<'de> RawDict<'de> {
    pub fn pop(&mut self, key: &Name) -> Option<RawValue<'de>> {
        self.0.remove(key)
    }

    pub fn pop_and_extract<T>(&mut self, key: &Name) -> Option<PResult<T>>
    where
        T: Extract<'de>,
    {
        let value = self.pop(key)?;
        Some(value.extract())
    }
}

impl<'de> Extract<'de> for RawDict<'de> {
    fn extract(input: &mut &'de BStr) -> PResult<Self> {
        let inner = extract(input)?;
        Ok(Self(inner))
    }

    fn recognize(input: &mut &'de BStr) -> PResult<&'de [u8]> {
        DoubleAngles::recognize(input)
    }
}

impl<'de, K, V> FromIterator<(K, V)> for RawDict<'de>
where
    K: Into<Name>,
    V: Into<RawValue<'de>>,
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let inner = iter
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
        Self(inner)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(b"+200")]
    #[case(b"+200")]
    #[case(b"/Name")]
    #[case(b"(string)")]
    #[case(b"<</Key1 true/Key2 (test)>>")]
    #[case(b"<F3BB>")]
    #[case(b"[1 2 3 4 true]")]
    fn raw_value(#[case] input: &[u8]) {
        let RawValue(value) = extract(&mut input.as_ref()).unwrap();
        assert_eq!(input, value.as_ref())
    }

    #[test]
    fn raw_dict() {
        let mut expected: RawDict = vec![
            ("Key1", b"true".as_slice()),
            ("Key2", b"false".as_slice()),
            ("Key3", b"42".as_slice()),
        ]
        .into_iter()
        .collect();

        let input: &[u8] = b"<</Key1 true/Key2   false   /Key3 42>>";
        let result = RawDict::extract(&mut input.as_ref()).unwrap();

        assert_eq!(result, expected);

        assert_eq!(
            expected.pop(&b"Key1".into()),
            Some(b"true".as_slice().into())
        );

        assert_eq!(expected.pop(&b"inexistant".into()), None);
    }
}