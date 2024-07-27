use livre_extraction::{Extract, HexBytes, Name, Reference};
use livre_utilities::{parse_real, take_whitespace};
use nom::combinator::recognize;

use serde::de::{
    self, DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, VariantAccess,
    Visitor,
};
use serde::Deserialize;

use paste::paste;

use crate::{Error, Result};

pub struct Deserializer<'de> {
    // This string starts with the input data and characters are truncated off
    // the beginning as data is parsed.
    input: &'de [u8],
}

impl<'de> Deserializer<'de> {
    pub fn new(input: &'de [u8]) -> Self {
        Self { input }
    }
    // By convention, `Deserializer` constructors are named like `from_xyz`.
    // That way basic use cases are satisfied by something like
    // `serde_json::from_str(...)` while advanced use cases that require a
    // deserializer can make one with `serde_json::Deserializer::from_str(...)`.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(input: &'de str) -> Self {
        Self::new(input.as_bytes())
    }

    pub fn from_bytes(input: &'de [u8]) -> Self {
        Self::new(input)
    }
}

// By convention, the public API of a Serde deserializer is one or more
// `from_xyz` methods such as `from_str`, `from_bytes`, or `from_reader`
// depending on what Rust types the deserializer is able to consume as input.
//
// This basic deserializer supports only `from_str`.
pub fn from_str<'a, T>(s: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_str(s);
    let t = T::deserialize(&mut deserializer)?;
    if deserializer.input.is_empty() {
        Ok(t)
    } else {
        Err(Error::TrailingCharacters)
    }
}

pub fn from_bytes<'a, T>(s: &'a [u8]) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_bytes(s);
    let t = T::deserialize(&mut deserializer)?;
    if deserializer.input.is_empty() {
        Ok(t)
    } else {
        Err(Error::TrailingCharacters)
    }
}

pub fn from_bytes_prefix<'a, T>(s: &'a [u8]) -> Result<(&'a [u8], T)>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_bytes(s);
    let t = T::deserialize(&mut deserializer)?;
    Ok((deserializer.input, t))
}

impl<'de> Deserializer<'de> {
    fn peek(&self) -> Result<u8> {
        self.input.first().copied().ok_or(Error::Eof)
    }
    fn next(&mut self) -> Result<u8> {
        let next = self.peek()?;
        self.input = &self.input[1..];
        Ok(next)
    }
    fn peek_n(&mut self, n: usize) -> Result<&[u8]> {
        if n > self.input.len() {
            Err(Error::Eof)
        } else {
            let result = &self.input[..n];
            Ok(result)
        }
    }

    fn next_n(&mut self, n: usize) -> Result<&[u8]> {
        if n > self.input.len() {
            Err(Error::Eof)
        } else {
            let result = &self.input[..n];
            self.input = &self.input[n..];
            Ok(result)
        }
    }

    fn remove_whitespace(&mut self) {
        let (input, _) = take_whitespace(self.input).unwrap();
        self.input = input;
    }

    fn parse<T: Extract<'de>>(&mut self) -> Result<T> {
        self.remove_whitespace();

        let (input, result) = T::extract(self.input).map_err(Error::custom)?;
        self.input = input;
        Ok(result)
    }

    fn parse_with_error<T: Extract<'de>>(&mut self, error: Error) -> Result<T> {
        self.remove_whitespace();

        let (input, result) = T::extract(self.input).map_err(|_| error)?;
        self.input = input;
        Ok(result)
    }
}

macro_rules! parse_with_error {
    ($name:ident, $error:path) => {
        paste! {
            fn [<deserialize_ $name:lower>]<V>(self, visitor: V) -> Result<V::Value>
            where
                V: Visitor<'de>,
            {
                let result = self.parse_with_error($error)?;
                visitor.[<visit_ $name:lower>](result)
            }
        }
    };
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.remove_whitespace();

        match self.peek()? {
            b'n' => self.deserialize_unit(visitor),
            b't' | b'f' => self.deserialize_bool(visitor),
            b'(' | b'/' => self.deserialize_string(visitor),
            b'-' | b'+' | b'.' | b'0'..=b'9' => {
                dbg!(String::from_utf8_lossy(self.input));
                if let Ok((input, reference)) = recognize(Reference::extract)(self.input) {
                    self.input = input;
                    visitor.visit_borrowed_bytes(reference)
                } else if let Ok((input, float)) = parse_real(self.input) {
                    self.input = input;
                    visitor.visit_f32(float)
                } else {
                    self.deserialize_i64(visitor)
                }
            }
            b'[' => self.deserialize_seq(visitor),
            b'<' => {
                if self.peek_n(2).is_ok_and(|peek| peek == b"<<") {
                    self.deserialize_map(visitor)
                } else {
                    self.deserialize_byte_buf(visitor)
                }
            }
            _ => self.deserialize_bytes(visitor),
        }
    }

    parse_with_error!(bool, Error::ExpectedBoolean);

    parse_with_error!(i8, Error::ExpectedInteger);
    parse_with_error!(i16, Error::ExpectedInteger);
    parse_with_error!(i32, Error::ExpectedInteger);
    parse_with_error!(i64, Error::ExpectedInteger);
    parse_with_error!(i128, Error::ExpectedInteger);

    parse_with_error!(u8, Error::ExpectedInteger);
    parse_with_error!(u16, Error::ExpectedInteger);
    parse_with_error!(u32, Error::ExpectedInteger);
    parse_with_error!(u64, Error::ExpectedInteger);
    parse_with_error!(u128, Error::ExpectedInteger);

    parse_with_error!(f32, Error::ExpectedFloat);
    parse_with_error!(f64, Error::ExpectedFloat);

    // Deserialization of `str` is tricky because of string escaping.
    // Useful reference for future use: <https://github.com/serde-rs/json/blob/master/src/de.rs#L1516>
    fn deserialize_str<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    // For now, we only deserialize to owned string.
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.remove_whitespace();

        let s = match self.peek()? {
            b'(' => self.parse_with_error::<String>(Error::ExpectedString)?,
            b'/' => self.parse_with_error::<Name>(Error::ExpectedName)?.into(),
            _ => return Err(Error::ExpectedString),
        };

        visitor.visit_string(s)
    }

    /// Deserialize bytes.
    ///
    /// We use bytes as a catch-all mechanism for deserializing non-trivial types.
    fn deserialize_bytes<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.remove_whitespace();

        let (input, reference) =
            recognize(Reference::extract)(self.input).map_err(|_| Error::Syntax)?;
        self.input = input;
        visitor.visit_borrowed_bytes(reference)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let HexBytes(bytes) = self.parse()?;
        visitor.visit_byte_buf(bytes)
    }

    // The PDF specification defines the `null` object, which may represent an
    // absent optional.
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.remove_whitespace();

        if self.input.starts_with(b"null") {
            self.input = &self.input[b"null".len()..];
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    // In Serde, unit means an anonymous value containing no data.
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.remove_whitespace();

        if self.input.starts_with(b"null") {
            self.input = &self.input[b"null".len()..];
            visitor.visit_unit()
        } else {
            Err(Error::ExpectedNull)
        }
    }

    // Unit struct means a named value containing no data.
    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    // As is done here, serializers are encouraged to treat newtype structs as
    // insignificant wrappers around the data they contain. That means not
    // parsing anything other than the contained value.
    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    // Deserialization of compound types like sequences and maps happens by
    // passing the visitor an "Access" object that gives it the ability to
    // iterate through the data contained in the sequence.
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.remove_whitespace();

        // Parse the opening bracket of the sequence.
        if self.next()? == b'[' {
            // Give the visitor access to each element of the sequence.
            let value = visitor.visit_seq(Accessor::new(self))?;
            // Parse the closing bracket of the sequence.
            if self.next()? == b']' {
                Ok(value)
            } else {
                Err(Error::ExpectedArrayEnd)
            }
        } else {
            Err(Error::ExpectedArray)
        }
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(TupleAccessor::new(self, len))
    }

    // Tuple structs look just like sequences in PDFs.
    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    // Much like `deserialize_seq` but calls the visitors `visit_map` method
    // with a `MapAccess` implementation, rather than the visitor's `visit_seq`
    // method with a `SeqAccess` implementation.
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.remove_whitespace();

        // Parse the opening brace of the map.
        if self.next_n(2)? == b"<<" {
            // Give the visitor access to each entry of the map.
            let value = visitor.visit_map(Accessor::new(self))?;
            // Parse the closing brace of the map.
            if self.next_n(2)? == b">>" {
                Ok(value)
            } else {
                Err(Error::ExpectedMapEnd)
            }
        } else {
            Err(Error::ExpectedMap)
        }
    }

    // Structs look just like maps in PDFs.
    //
    // Notice the `fields` parameter - a "struct" in the Serde data model means
    // that the `Deserialize` implementation is required to know what the fields
    // are before even looking at the input data. Any key-value pairing in which
    // the fields cannot be known ahead of time is probably a map.
    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.remove_whitespace();

        if self.peek()? == b'/' {
            // Visit a unit variant.
            let Name(variant) = self.parse_with_error(Error::ExpectedString)?;
            visitor.visit_enum(variant.into_deserializer())
        } else if self.next_n(2)? == b"<<" {
            // Visit a newtype variant, tuple variant, or struct variant.
            let value = visitor.visit_enum(Enum::new(self))?;
            // Parse the matching close brace.
            if self.next_n(2)? == b">>" {
                Ok(value)
            } else {
                Err(Error::ExpectedMapEnd)
            }
        } else {
            Err(Error::ExpectedEnum)
        }
    }

    // An identifier in Serde is the type that identifies a field of a struct or
    // the variant of an enum. In PDFs, struct fields and enum variants are
    // represented as strings.
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    // Like `deserialize_any` but indicates to the `Deserializer` that it makes
    // no difference which `Visitor` method is called because the data is
    // ignored.
    //
    // Some deserializers are able to implement this more efficiently than
    // `deserialize_any`, for example by rapidly skipping over matched
    // delimiters without paying close attention to the data in between.
    //
    // Some formats are not able to implement this at all. Formats that can
    // implement `deserialize_any` and `deserialize_ignored_any` are known as
    // self-describing.
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_char<V>(self, _visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!();
    }
}

struct Accessor<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> Accessor<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Self { de }
    }
}

// `Accessor` is provided to the `Visitor` to give it the ability to iterate
// through elements of the sequence.
impl<'de, 'a> SeqAccess<'de> for Accessor<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        self.de.remove_whitespace();

        // Check if there are no more elements.
        if self.de.peek()? == b']' {
            return Ok(None);
        }
        // Deserialize an array element.
        seed.deserialize(&mut *self.de).map(Some)
    }
}

// `Accessor` is provided to the `Visitor` to give it the ability to iterate
// through entries of the map.
impl<'de, 'a> MapAccess<'de> for Accessor<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        self.de.remove_whitespace();

        // Check if there are no more entries.
        if self.de.peek_n(2)? == b">>" {
            return Ok(None);
        }
        // Deserialize a map key.
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        self.de.remove_whitespace();
        seed.deserialize(&mut *self.de)
    }
}

struct TupleAccessor<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    len: usize,
}

impl<'a, 'de: 'a> TupleAccessor<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>, len: usize) -> Self {
        Self { de, len }
    }
}

// `TupleAccessor` does not check for elements
impl<'de, 'a> SeqAccess<'de> for TupleAccessor<'a, 'de> {
    type Error = Error;

    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        // Check if there are no more elements.
        if self.len == 0 {
            return Ok(None);
        }

        self.de.remove_whitespace();

        // Decrement the number of expected elements
        self.len -= 1;

        // Deserialize an array element.
        seed.deserialize(&mut *self.de).map(Some)
    }
}

struct Enum<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> Enum<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Enum { de }
    }
}

// `EnumAccess` is provided to the `Visitor` to give it the ability to determine
// which variant of the enum is supposed to be deserialized.
//
// Note that all enum deserialization methods in Serde refer exclusively to the
// "externally tagged" enum representation.
impl<'de, 'a> EnumAccess<'de> for Enum<'a, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        // The `deserialize_enum` method parsed a `{` character so we are
        // currently inside of a map. The seed will be deserializing itself from
        // the key of the map.
        self.de.remove_whitespace();
        let val = seed.deserialize(&mut *self.de)?;

        Ok((val, self))
    }
}

// `VariantAccess` is provided to the `Visitor` to give it the ability to see
// the content of the single variant that it decided to deserialize.
impl<'de, 'a> VariantAccess<'de> for Enum<'a, 'de> {
    type Error = Error;

    // If the `Visitor` expected this variant to be a unit variant, the input
    // should have been the plain string case handled in `deserialize_enum`.
    fn unit_variant(self) -> Result<()> {
        Err(Error::ExpectedString)
    }

    // Newtype variants are represented in JSON as `{ NAME: VALUE }` so
    // deserialize the value here.
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        self.de.remove_whitespace();
        seed.deserialize(self.de)
    }

    // Tuple variants are represented in JSON as `{ NAME: [DATA...] }` so
    // deserialize the sequence of data here.
    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_tuple(self.de, len, visitor)
    }

    // Struct variants are represented in JSON as `{ NAME: { K: V, ... } }` so
    // deserialize the inner map here.
    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_map(self.de, visitor)
    }
}

#[cfg(test)]
mod tests {

    use std::{collections::HashMap, fmt::Debug};

    use indoc::indoc;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(b"18", 18u32)]
    #[case(b"+42", 42u8)]
    #[case(b"false", false)]
    #[case(b"   false", false)]
    #[case(b"true", true)]
    #[case(b"null", ())]
    #[case(b" -42", -42i64)]
    #[case(b"12", 12i32)]
    #[case(b"12", 12.0)]
    #[case(b"-42.42", -42.42f64)]
    #[case(b"-.42", -0.42f32)]
    #[case(b"(Test)", "Test".to_string())]
    #[case(b"/Test", "Test".to_string())]
    fn primitives<'de, T: Deserialize<'de> + PartialEq + Debug>(
        #[case] input: &'de [u8],
        #[case] expected: T,
    ) {
        assert_eq!(expected, from_bytes(input).unwrap());
    }

    #[rstest]
    #[case(b"[1 2   4]", vec![1, 2, 4])]
    #[case(b"[ 1]", vec![1])]
    #[case(b"<< /Test 1>>", vec![("Test".to_string(), 1)].into_iter().collect::<HashMap<String, i32>>())]
    fn containers<'de, T: Deserialize<'de> + PartialEq + Debug>(
        #[case] input: &'de [u8],
        #[case] expected: T,
    ) {
        assert_eq!(expected, from_bytes(input).unwrap());
    }

    #[derive(Deserialize, Debug, PartialEq)]
    #[serde(rename_all = "PascalCase")]
    struct Test {
        int: u32,
        float: f64,
        boolean: bool,
    }

    #[rstest]
    #[case(b"<< /Int 42/Float 42/Boolean true /Unknown [/Test]  >>", Test{ int: 42, float: 42.0, boolean: true })]
    #[case(
        indoc!{b"
            <<
                /Int 42
                /Float 42
                /Boolean true
                /Unknown [
                    /Test
                ]
            >>\
        "}, Test{ int: 42, float: 42.0, boolean: true })]
    fn structs<'de, T: Deserialize<'de> + PartialEq + Debug>(
        #[case] input: &'de [u8],
        #[case] expected: T,
    ) {
        assert_eq!(expected, from_bytes(input).unwrap());
    }

    #[rstest]
    #[case(b"(TEST) 42", ("TEST".to_string(), 42u8))]
    #[case(b"null 42", (None::<i32>, 42u8))]
    #[allow(clippy::approx_constant)]
    #[case(b"3.14 10", (3.14, 10))]
    fn tuples<'de, T: Deserialize<'de> + PartialEq + Debug>(
        #[case] input: &'de [u8],
        #[case] expected: T,
    ) {
        assert_eq!(expected, from_bytes(input).unwrap());
    }

    #[rstest]
    #[case(b"<</a 1/b 2>>", [("a".to_string(), 1), ("b".to_string(), 2)].into_iter().collect::<HashMap::<_, _>>())]
    #[case(b"<</test (it works!)>>", [("test".to_string(), "it works!".to_string())].into_iter().collect::<HashMap::<_, _>>())]
    fn maps<'de, T: Deserialize<'de> + PartialEq + Debug>(
        #[case] input: &'de [u8],
        #[case] expected: T,
    ) {
        assert_eq!(expected, from_bytes(input).unwrap());
    }

    #[derive(Debug, PartialEq, Deserialize)]
    enum TestEnum {
        Plain,
        Struct { a: u8 },
        Tuple(f32),
        DoubleTuple(i32, bool),
    }

    #[derive(Debug, PartialEq, Deserialize)]
    #[serde(tag = "type")]
    enum TestTaggedEnum {
        Plain,
        Struct { a: f32 },
        StructInt { b: i32 },
    }

    #[derive(Debug, PartialEq, Deserialize)]
    #[serde(untagged)]
    enum TestUntaggedEnum {
        Plain,
        Struct { a: f32 },
        StructInt { b: i32 },
    }

    #[rstest]
    #[case(b"/Plain", TestEnum::Plain)]
    #[case(b"<</Struct<</a 8>>>>", TestEnum::Struct { a: 8 })]
    #[case(b"<</Tuple -256.3>>", TestEnum::Tuple(-256.3))]
    #[case(b"<</DoubleTuple -2 true>>", TestEnum::DoubleTuple(-2, true))]
    fn enums(#[case] input: &[u8], #[case] expected: TestEnum) {
        assert_eq!(expected, from_bytes(input).unwrap());
    }

    #[rstest]
    #[case(b"<</type /Plain>>", TestTaggedEnum::Plain)]
    #[case(b"<</a 8 /type/Struct  >>", TestTaggedEnum::Struct { a: 8.0 })]
    fn tagged_enums(#[case] input: &[u8], #[case] expected: TestTaggedEnum) {
        assert_eq!(expected, from_bytes(input).unwrap());
    }

    #[rstest]
    #[case(b"null", TestUntaggedEnum::Plain)]
    #[case(b"<</a 8 >>", TestUntaggedEnum::Struct { a: 8.0 })]
    #[case(b"<</a 8.0 >>", TestUntaggedEnum::Struct { a: 8.0 })]
    #[case(b"<</b 8 >>", TestUntaggedEnum::StructInt { b: 8})]
    fn untagged_enums(#[case] input: &[u8], #[case] expected: TestUntaggedEnum) {
        assert_eq!(expected, from_bytes(input).unwrap());
    }
}
