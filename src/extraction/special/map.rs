use std::collections::HashMap;

use winnow::{
    ascii::multispace0,
    combinator::{iterator, peek, separated_pair, terminated, trace},
    dispatch,
    error::ContextError,
    token::{any, take_till},
    BStr, PResult, Parser,
};

use crate::{
    extraction::{
        extract,
        utilities::{Angles, Brackets, DoubleAngles, Parentheses},
    },
    Extract, FromRawDict,
};

use super::name::Name;

/// In PDFs, dictionary keys are [`Name`]s.
pub type Map<T> = HashMap<Name, T>;

impl<'de, T> FromRawDict<'de> for Map<T>
where
    T: Extract<'de>,
{
    fn from_raw_dict(dict: &mut RawDict<'de>) -> PResult<Self> {
        let mut map = Map::with_capacity(dict.0.len());

        for (key, value) in dict.0.drain() {
            // NOTE: this is debatable. The alternative would be to fail whenever there's
            // a value that cannot be extracted.
            // Let's try it out this way and see how it goes.
            if let Ok(value) = value.extract() {
                map.insert(key, value);
            }
        }

        Ok(map)
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

fn remove_trailing_spaces(input: &[u8]) -> &[u8] {
    let index = input.iter().rev().enumerate().find_map(|(i, b)| {
        if b" \t\r\n".contains(b) {
            None
        } else {
            Some(i)
        }
    });

    if let Some(i) = index {
        &input[..(input.len() - i)]
    } else {
        &input[..0]
    }
}

impl<'de> Extract<'de> for RawValue<'de> {
    fn extract(input: &mut &'de BStr) -> PResult<Self> {
        dispatch! {peek(any);
            b'/' => Name::recognize,
            b'[' => Brackets::recognize,
            b'(' => Parentheses::recognize,
            b'<' => Angles::recognize,
            // NOTE: provided we do not encounter a name *within a tuple*, this last case
            // handles every other option.
            _ => take_till(0.., b'/').map(remove_trailing_spaces),
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

    pub fn pop_required(&mut self, key: &Name) -> PResult<RawValue<'de>> {
        self.pop(key)
            .ok_or(winnow::error::ErrMode::Cut(ContextError::new()))
    }

    pub fn pop_and_extract<T>(&mut self, key: &Name) -> Option<PResult<T>>
    where
        T: Extract<'de>,
    {
        let value = self.pop(key)?;
        Some(value.extract())
    }

    pub fn pop_and_extract_required<T>(&mut self, key: &Name) -> PResult<T>
    where
        T: Extract<'de>,
    {
        let value = self.pop_required(key)?;
        value.extract()
    }
}

impl<'de> Extract<'de> for RawDict<'de> {
    fn extract(input: &mut &'de BStr) -> PResult<Self> {
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

            Ok(Self(map))
        })
        .parse_next(input)
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

#[derive(Debug, PartialEq, Clone, Copy, Eq)]
pub struct Nil;

impl FromRawDict<'_> for Nil {
    fn from_raw_dict(_: &mut crate::extraction::special::RawDict<'_>) -> PResult<Self> {
        Ok(Nil)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(b"test ", b"test")]
    #[case(b"test\r\n", b"test")]
    #[case(b"test\n", b"test")]
    fn trailing_spaces(#[case] input: &[u8], #[case] expected: &[u8]) {
        assert_eq!(expected, remove_trailing_spaces(input))
    }

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
