use std::collections::HashMap;

use livre_extraction::{extract, parse, DoubleAngles, Extract};
use livre_serde::extract_deserialize;
use livre_utilities::take_whitespace;
use nom::{branch::alt, bytes::complete::tag, sequence::tuple, IResult};
use serde::{de::Visitor, Deserialize};

use paste::paste;

use super::Stream;
use crate::Reference;

#[derive(Debug, Clone, PartialEq)]
pub enum Object {
    Null,
    Boolean(bool),
    Integer(i64),
    Real(f32),
    String(String),
    Array(Vec<Object>),
    Dictionary(HashMap<String, Object>),
    Stream(Stream<HashMap<String, Object>>),
    Reference(Reference),
}

macro_rules! impl_from {
    ($name:ty => $variant:ident) => {
        impl From<$name> for Object {
            fn from(v: $name) -> Self {
                Self::$variant(v.into())
            }
        }
    };
}

impl_from!(bool => Boolean);
impl_from!(i32 => Integer);
impl_from!(f32 => Real);

macro_rules! impl_visit {
    ($name:ident, $type:ident -> $variant:ident) => {
        paste! {
            fn [<visit_ $name:lower>]<E>(self, v: $type) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Object::$variant(v))
            }
        }
    };
    ($name:ident -> $variant:ident) => {
        impl_visit!($name, $name -> $variant);
    };
}

struct ObjectVisitor;

impl<'de> Visitor<'de> for ObjectVisitor {
    type Value = Object;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a PDF object")
    }

    impl_visit!(bool -> Boolean);
    impl_visit!(i64 -> Integer);
    impl_visit!(f32 -> Real);

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Object::String(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Object::String(v.into()))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Object::Null)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Object::Null)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut array = Vec::new();

        while let Some(item) = seq.next_element::<Object>()? {
            array.push(item);
        }

        Ok(Object::Array(array))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut res = HashMap::new();

        while let Some((k, v)) = map.next_entry::<String, Object>()? {
            res.insert(k, v);
        }

        Ok(Object::Dictionary(res))
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let reference = parse(v).map_err(E::custom)?;
        Ok(Object::Reference(reference))
    }
}

impl<'de> Deserialize<'de> for Object {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(ObjectVisitor)
    }
}

impl Object {
    /// Avoid parsing the map before knowing weather we're in a stream or a dict.
    fn extract_stream(input: &[u8]) -> IResult<&[u8], Self> {
        // Fail early if there's no stream. At this point we've only paid for recognizing
        // the double brackets, which is quite cheap
        let _ = tuple((DoubleAngles::extract, take_whitespace, tag("stream")))(input)?;

        let (input, stream) = extract(input)?;
        Ok((input, Self::Stream(stream)))
    }
}

impl<'input> Extract<'input> for Object {
    fn extract(input: &'input [u8]) -> IResult<&'input [u8], Self> {
        alt((Self::extract_stream, extract_deserialize))(input)
    }
}

#[cfg(test)]
mod tests {

    use indoc::indoc;
    use rstest::rstest;

    use super::*;

    macro_rules! obj {
        (!) => {
            HashMap::new()
        };
        () => {
            Object::Null
        };
        (b:$val:literal) => {
            Object::Boolean($val)
        };
        (i:$val:literal) => {
            Object::Integer($val)
        };
        (f:$val:literal) => {
            Object::Real($val)
        };
        (t:$val:literal) => {
            Object::String($val.to_string())
        };
        ($($o:expr),+ $(,)?) => {
            Object::Array(vec![$($o),+].into())
        };
        (r:$obj:literal $gen:literal) => {
            Object::Reference(Reference{object: $obj, generation: $gen})
        };
        (map:$($k:literal $v:expr),+ $(,)?) => {
            vec![$(($k.to_string(), $v)),+].into_iter().collect()
        };
        ($($k:literal $v:expr),+ $(,)?) => {
            Object::Dictionary(vec![$(($k.to_string(), $v)),+].into_iter().collect())
        };
        (s:$decoded:expr, $structured:expr) => {
            Object::Stream(Stream{decoded: $decoded.into(), structured: $structured})
        };
    }

    #[rstest]
    #[case(b"null", obj!())]
    #[case(b"true", obj!(b:true))]
    #[case(b"false", obj!(b:false))]
    #[case(b"0 10 R", obj!(r:0 10))]
    #[case(b"(Longtemps, je me suis)", obj!(t:"Longtemps, je me suis"))]
    #[case(b"/Test", obj!(t:"Test"))]
    // #[case(b"<00A01>", obj!(h:[0, 160, 16]))]
    // #[case(b"<00A010>", obj!(h:[0, 160, 16]))]
    #[case(b"<</FirstKey/Test/AnotherKey 2.>>", obj!("FirstKey" obj!(t:"Test"), "AnotherKey" obj!(f:2.0)))]
    #[case(indoc!{b"
        <<
            /DA(/Helv 0 Tf 0 g )
            /Fields[]
        >>
    "}, obj!("DA" obj!(t:"/Helv 0 Tf 0 g "), "Fields" Object::Array(vec![])))]
    #[case(
        indoc! {b"
            <</Length 10>> stream
            0123456789
            endstream
        "},
        obj!(s:b"0123456789", obj!(!))
    )]
    #[case(
        indoc! {b"
            <</Length 10/Test/Test/int -42>> stream
            0123456789
            endstream
        "},
        obj!(s:b"0123456789", obj!(map: "Test" obj!(t:"Test"), "int" obj!(i:-42)))
    )]
    fn object(#[case] input: &[u8], #[case] expected: Object) {
        let (_, obj) = extract(input).unwrap();
        assert_eq!(expected, obj);
    }
}
