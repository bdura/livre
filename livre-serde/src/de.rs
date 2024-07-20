use livre_extraction::{Extract, HexBytes, Name, Reference};
use livre_utilities::take_whitespace;
use nom::combinator::recognize;
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::Deserialize;

use paste::paste;

use crate::{Error, Result};

pub struct Deserializer<'de> {
    // This string starts with the input data and characters are truncated off
    // the beginning as data is parsed.
    input: &'de [u8],
}

impl<'de> Deserializer<'de> {
    // By convention, `Deserializer` constructors are named like `from_xyz`.
    // That way basic use cases are satisfied by something like
    // `serde_json::from_str(...)` while advanced use cases that require a
    // deserializer can make one with `serde_json::Deserializer::from_str(...)`.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(input: &'de str) -> Self {
        Deserializer {
            input: input.as_bytes(),
        }
    }

    pub fn from_bytes(input: &'de [u8]) -> Self {
        Deserializer { input }
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
        let (input, result) = T::extract(self.input).map_err(|e| Error::Message(e.to_string()))?;
        self.input = input;
        Ok(result)
    }
}

macro_rules! deserialize_using_parse {
    ($name:ident) => {
        paste! {
            fn [<deserialize_ $name:lower>]<V>(self, visitor: V) -> Result<V::Value>
            where
                V: Visitor<'de>,
            {
                let result = self.parse()?;
                visitor.[<visit_ $name:lower>](result)
            }
        }
    };
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    // Look at the input data to decide what Serde data model type to
    // deserialize as. Not all data formats are able to support this operation.
    // Formats that support `deserialize_any` are known as self-describing.
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.peek()? {
            b'n' => self.deserialize_unit(visitor),
            b't' | b'f' => self.deserialize_bool(visitor),
            b'(' | b'/' => self.deserialize_string(visitor),
            b'0'..=b'9' => self.deserialize_u64(visitor),
            b'-' => self.deserialize_i64(visitor),
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

    deserialize_using_parse!(bool);

    deserialize_using_parse!(i8);
    deserialize_using_parse!(i16);
    deserialize_using_parse!(i32);
    deserialize_using_parse!(i64);
    deserialize_using_parse!(i128);

    deserialize_using_parse!(u8);
    deserialize_using_parse!(u16);
    deserialize_using_parse!(u32);
    deserialize_using_parse!(u64);
    deserialize_using_parse!(u128);

    deserialize_using_parse!(f32);
    deserialize_using_parse!(f64);

    // Refer to the "Understanding deserializer lifetimes" page for information
    // about the three deserialization flavors of strings in Serde.
    fn deserialize_str<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let s = match self.peek()? {
            b'(' => self.parse::<String>()?,
            b'/' => self.parse::<Name>()?.into(),
            _ => return Err(Error::ExpectedString),
        };

        visitor.visit_string(s)
    }

    // deserialize_using_parse!(bytes);

    /// We use bytes as a catch-all mechanism for deserializing non-trivial types.
    fn deserialize_bytes<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
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

    // An absent optional is represented as the JSON `null` and a present
    // optional is represented as just the contained value.
    //
    // As commented in `Serializer` implementation, this is a lossy
    // representation. For example the values `Some(())` and `None` both
    // serialize as just `null`. Unfortunately this is typically what people
    // expect when working with JSON. Other formats are encouraged to behave
    // more intelligently if possible.
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
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

    // Tuples look just like sequences in JSON. Some formats may be able to
    // represent tuples more efficiently.
    //
    // As indicated by the length parameter, the `Deserialize` implementation
    // for a tuple in the Serde data model is required to know the length of the
    // tuple before even looking at the input data.
    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    // Tuple structs look just like sequences in JSON.
    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    // Much like `deserialize_seq` but calls the visitors `visit_map` method
    // with a `MapAccess` implementation, rather than the visitor's `visit_seq`
    // method with a `SeqAccess` implementation.
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
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

    // Structs look just like maps in PDF.
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
        _visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    // An identifier in Serde is the type that identifies a field of a struct or
    // the variant of an enum. In PDFs, struct fields and enum variants are
    // represented as strings. In other formats they may be represented as
    // numeric indices.
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

// `SeqAccess` is provided to the `Visitor` to give it the ability to iterate
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

// `MapAccess` is provided to the `Visitor` to give it the ability to iterate
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use livre_extraction::TypedReference;
    use serde::Deserialize;

    #[test]
    fn test_struct() {
        #[derive(Deserialize, PartialEq, Debug)]
        #[serde(rename_all = "PascalCase")]
        struct Test {
            int: u32,
            map: HashMap<String, i32>,
            reference: TypedReference<i32>,
        }

        let j = "<</Int 42 /Seq [1 2 3 ] /Map <</test 42 /test2 -12>>\n\n/Reference    0 10 R>>";
        let expected = Test {
            int: 42,
            map: vec![("test".to_string(), 42), ("test2".to_string(), -12)]
                .into_iter()
                .collect(),
            reference: TypedReference::new(0, 10),
        };
        assert_eq!(expected, dbg!(from_str(j)).unwrap());
    }
}
