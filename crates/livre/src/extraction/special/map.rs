use std::collections::HashMap;

use winnow::{
    combinator::{eof, iterator, peek, separated_pair, terminated, trace},
    dispatch,
    error::{ContextError, ErrMode},
    token::{any, take_till},
    BStr, ModalResult, Parser,
};

use crate::{
    extraction::{
        extract, multicomment0,
        utilities::{recognize_angled, Brackets, DoubleAngles, Parentheses},
        Extract, FromRawDict,
    },
    follow_refs::{Build, BuildFromRawDict, Builder},
};

use super::name::Name;

/// In PDFs, dictionary keys are [`Name`]s.
pub type Map<T> = HashMap<Name, T>;

impl<'de, T> FromRawDict<'de> for Map<T>
where
    T: Extract<'de>,
{
    fn from_raw_dict(dict: &mut RawDict<'de>) -> ModalResult<Self> {
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
fn parse_key_value<'de, T>(input: &mut &'de BStr) -> ModalResult<(Name, T)>
where
    T: Extract<'de>,
{
    trace(
        "livre-key-value",
        terminated(
            separated_pair(Name::extract, multicomment0, T::extract),
            multicomment0,
        ),
    )
    .parse_next(input)
}

/// A container for **any** PDF type that may appear, to be parsed by a
/// dedicated parser after the facts.
///
/// In effect, it is merely a pointer to the start of the defered object within
/// the input stream. The point of having a dedicated type for that lies within
/// the specific extraction logic: all the [`RawValue`] knows how to do is
/// recognize the next PDF object, without explicitly parsing it.
///
/// `RawValue` also provides an [`extract`](Self::extract) method to simplify
/// the extraction ot the actual value.
///
/// `RawValue` is the main building block for the [`RawDict`] type.
///
#[derive(Debug, PartialEq)]
pub struct RawValue<'de>(pub &'de BStr);

impl<'de> RawValue<'de> {
    /// Extract the raw value into a strongly typed object.
    ///
    /// The whole value must be read: `10 0 R` is not the integer `10`.
    pub fn extract<T>(mut self) -> ModalResult<T>
    where
        T: Extract<'de>,
    {
        terminated(extract, eof).parse_next(&mut self.0)
    }

    /// Build the raw value into a strongly typed object.
    ///
    /// The whole value must be read, as with [`extract`](Self::extract).
    pub fn build<T, B>(mut self, builder: &B) -> ModalResult<T>
    where
        T: Build,
        B: Builder,
    {
        terminated(|i: &mut &BStr| T::build(i, builder), eof).parse_next(&mut self.0)
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
    fn extract(input: &mut &'de BStr) -> ModalResult<Self> {
        dispatch! {peek(any);
            b'/' => Name::recognize,
            b'[' => Brackets::recognize,
            b'(' => Parentheses::recognize,
            b'<' => recognize_angled,
            // Catch-all for values that start with bytes other than the dispatch arms above:
            // numbers (e.g. `42`, `-3.14`), booleans (`true`, `false`), and indirect references
            // (`1 0 R`). These all end at the next `/` (the start of the next key), at a `%`
            // comment, or at the end of the dictionary.
            // TODO: replace this with per-type dispatch once all value kinds are covered, so that
            // truly invalid input is rejected rather than silently consumed.
            _ => take_till(0.., (b'/', b'%')).map(remove_trailing_spaces),
        }
        .map(Self::from)
        .parse_next(input)
    }
}

/// A dictionary instance that keeps values in their raw form, i.e. we only
/// detect the bounds of each value without actually parsing them.
///
/// The intended workflow is as follows:
///
/// 1. Build a [`RawDict`] by iterating through key-value pairs.
/// 2. Extract the actual structured type using [`FromRawDict`].
#[derive(Debug, PartialEq)]
pub struct RawDict<'de>(Map<RawValue<'de>>);

impl<'de> RawDict<'de> {
    pub fn pop(&mut self, key: &Name) -> Option<RawValue<'de>> {
        self.0.remove(key)
    }

    pub fn pop_and_extract<T>(&mut self, key: &Name) -> Option<ModalResult<T>>
    where
        T: Extract<'de>,
    {
        let value = self.pop(key)?;
        Some(value.extract())
    }

    pub fn pop_and_build<T, B>(&mut self, key: &Name, builder: &B) -> ModalResult<Option<T>>
    where
        T: Build,
        B: Builder,
    {
        self.pop(key).map(|value| value.build(builder)).transpose()
    }
}

impl<'de> Extract<'de> for RawDict<'de> {
    fn extract(input: &mut &'de BStr) -> ModalResult<Self> {
        trace("livre-map", |i: &mut &'de BStr| {
            let DoubleAngles(mut inside) = extract(i)?;

            // Remove leading spaces
            multicomment0(&mut inside)?;

            // `parse_key_value` includes trailing spaces
            let mut it = iterator(&mut inside, parse_key_value);

            let map = it.collect();
            it.finish()?;

            // Fail if there is unconsumed input left inside the `<<...>>` block.
            // At this point the opening delimiters have already been consumed, so we
            // escalate to `Cut` to prevent spurious backtracking in callers.
            if !inside.is_empty() {
                return Err(ErrMode::Cut(ContextError::new()));
            }

            Ok(Self(map))
        })
        .parse_next(input)
    }

    fn recognize(input: &mut &'de BStr) -> ModalResult<&'de [u8]> {
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
    fn from_raw_dict(_: &mut crate::extraction::special::RawDict<'_>) -> ModalResult<Self> {
        Ok(Nil)
    }
}

impl BuildFromRawDict for Nil {
    fn build_from_raw_dict<B>(_dict: &mut RawDict<'_>, _builder: &B) -> ModalResult<Self>
    where
        B: Builder,
    {
        Ok(Self)
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
    #[case(b"[(a]b) ([)]")]
    #[case(b"<</T (a>b)>>")]
    #[case(b"<</K [<</B (>>)>>] /H <ab>>>")]
    fn raw_value(#[case] input: &[u8]) {
        let RawValue(value) = extract(&mut input.as_ref()).unwrap();
        assert_eq!(input, value.as_ref())
    }

    #[test]
    fn raw_value_must_be_read_whole() {
        let value = RawValue::from(b"10 0 R".as_slice());
        assert!(value.extract::<usize>().is_err());
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

    #[test]
    fn raw_dict_with_comments() {
        let expected: RawDict = vec![
            ("Key1", b"true".as_slice()),
            ("Key2", b"false".as_slice()),
            ("Key3", b"42".as_slice()),
            ("Key4", b"1 0 R".as_slice()),
        ]
        .into_iter()
        .collect();

        let input: &[u8] =
            b"<< % leading\n/Key1 % before\ntrue/Key2 false% after\n/Key3 42 % see /Key2\n/Key4 1 0 R%\n>>";
        let result = RawDict::extract(&mut input.as_ref()).unwrap();

        assert_eq!(result, expected);
    }

    #[test]
    fn raw_dict_with_delimiters_in_strings_and_comments() {
        let expected: RawDict = vec![
            ("T", b"(a>b)".as_slice()),
            ("A", b"[(a]b) ([)]".as_slice()),
            ("H", b"<ab>".as_slice()),
            ("D", b"<</K (>>)>>".as_slice()),
            ("N", b"1".as_slice()),
        ]
        .into_iter()
        .collect();

        let input: &[u8] = b"<</T (a>b) /A [(a]b) ([)] /H <ab> /D <</K (>>)>> /N 1 % x > y\n>>";
        let result = RawDict::extract(&mut input.as_ref()).unwrap();

        assert_eq!(result, expected);
    }
}
