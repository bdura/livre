use std::collections::HashMap;

use nom::{
    branch::alt,
    bytes::complete::{tag, take, take_till},
    character::complete::{digit0, digit1},
    combinator::{cut, map, opt, recognize},
    error::{Error, ErrorKind, ParseError},
    multi::many0,
    sequence::{delimited, pair, separated_pair, tuple, Tuple},
    Err, IResult,
};
use strum::IntoStaticStr;

use super::utilities::{
    parse_octal, take_eol, take_eol_no_r, take_whitespace, take_whitespace1, take_within_balanced,
};
use crate::{
    error::{ParsingError, Result},
    filters::{Filter, Filtering},
    parsers::utilities::{
        is_space_or_newline, parse_hexadecimal_bigram, parse_string_with_escapes,
    },
};

#[derive(Debug, Clone, PartialEq)]
pub struct Stream {
    pub filters: Vec<Filter>,
    pub stream: Vec<u8>,
}

impl Stream {
    pub fn decode(&self) -> Vec<u8> {
        let mut stream = self.stream.clone();
        for filter in &self.filters {
            stream = filter.decode(&stream).unwrap();
        }
        stream
    }
}

#[derive(Debug, Clone, PartialEq, IntoStaticStr)]
pub enum Object {
    Null,
    Boolean(bool),
    Integer(i32),
    Real(f32),
    LiteralString(String),
    HexString(Vec<u8>),
    Name(String),
    Array(Vec<Object>),
    Dictionary(HashMap<String, Object>),
    Stream(Stream),
    Reference(Reference),
}

impl TryFrom<Object> for bool {
    type Error = ParsingError;

    fn try_from(value: Object) -> Result<Self> {
        if let Object::Boolean(b) = value {
            Ok(b)
        } else {
            Err(ParsingError::UnexpectedType {
                expected: "Boolean",
                got: value.into(),
            })
        }
    }
}

impl TryFrom<&Object> for bool {
    type Error = ParsingError;

    fn try_from(value: &Object) -> Result<Self> {
        if let Object::Boolean(b) = value {
            Ok(*b)
        } else {
            Err(ParsingError::UnexpectedType {
                expected: "Boolean",
                got: value.into(),
            })
        }
    }
}

impl TryFrom<Object> for usize {
    type Error = ParsingError;

    fn try_from(value: Object) -> Result<Self> {
        if let Object::Integer(i) = value {
            Ok(i as usize)
        } else {
            Err(ParsingError::UnexpectedType {
                expected: "Integer",
                got: value.into(),
            })
        }
    }
}

impl TryFrom<&Object> for usize {
    type Error = ParsingError;

    fn try_from(value: &Object) -> Result<Self> {
        if let Object::Integer(i) = value {
            Ok(*i as usize)
        } else {
            Err(ParsingError::UnexpectedType {
                expected: "Integer",
                got: value.into(),
            })
        }
    }
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Reference {
    obj: usize,
    gen: usize,
}

impl Reference {
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, (obj, _, gen)) = (digit1, tag(b" "), digit1).parse(input)?;

        // SAFETY: obj is guaranteed to contain digits
        let obj: usize = unsafe { std::str::from_utf8_unchecked(obj).parse().unwrap() };
        // SAFETY: gen is guaranteed to contain digits
        let gen: usize = unsafe { std::str::from_utf8_unchecked(gen).parse().unwrap() };

        Ok((input, Self { obj, gen }))
    }
}

impl Object {
    fn parse_boolean(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, b) = alt((tag(b"true"), tag(b"false")))(input)?;

        let obj = match b {
            b"true" => Self::Boolean(true),
            b"false" => Self::Boolean(false),
            _ => unreachable!("The tags should only match true or false."),
        };

        Ok((input, obj))
    }

    fn parse_sign(input: &[u8]) -> IResult<&[u8], &[u8]> {
        alt((tag(b"+"), tag(b"-")))(input)
    }

    fn parse_integer(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, num) = recognize(pair(opt(Self::parse_sign), digit1))(input)?;

        // SAFETY: we know for a fact that `num` only includes ascii characters
        let num = unsafe { std::str::from_utf8_unchecked(num) };

        Ok((input, Self::Integer(num.parse().unwrap())))
    }

    fn parse_unsigned_real(input: &[u8]) -> IResult<&[u8], &[u8]> {
        alt((
            recognize(separated_pair(digit1, tag(b"."), digit0)),
            recognize(separated_pair(digit0, tag(b"."), digit1)),
        ))(input)
    }

    fn parse_real(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, num) =
            recognize(pair(opt(Self::parse_sign), Self::parse_unsigned_real))(input)?;

        // SAFETY: we know for a fact that `num` only includes ascii characters
        let num = unsafe { std::str::from_utf8_unchecked(num) };

        Ok((input, Self::Real(num.parse().unwrap())))
    }

    /// Parse real or integer object.
    ///
    /// Makes sure to test real before integer.
    fn parse_numeric(input: &[u8]) -> IResult<&[u8], Self> {
        alt((Self::parse_real, Self::parse_integer))(input)
    }

    fn parse_literal_string(input: &[u8]) -> IResult<&[u8], Self> {
        fn escaped_char(input: &[u8]) -> IResult<&[u8], Option<char>> {
            let (input, _) = take(1usize)(input)?;

            alt((
                map(tag(b"\n"), |_| None),
                map(tag(b"n"), |_| Some('\n')),
                map(tag(b"r"), |_| Some('\r')),
                map(tag(b"t"), |_| Some('\t')),
                map(tag(b"b"), |_| Some('\u{21A1}')),
                map(tag(b"f"), |_| Some('\u{232B}')),
                map(tag(b"("), |_| Some('(')),
                map(tag(b")"), |_| Some(')')),
                map(tag(b"\\"), |_| Some('\\')),
                map(parse_octal, |n| Some(n as char)),
            ))(input)
        }

        let (input, value) = take_within_balanced(b'(', b')')(input)?;
        let (d, lines) = many0(parse_string_with_escapes(b'\\', escaped_char))(value)?;
        assert!(d.is_empty());
        Ok((input, Self::LiteralString(lines.join(""))))
    }

    fn parse_hexadecimal_bigram(input: &[u8]) -> IResult<&[u8], u8> {
        fn inner(input: &[u8]) -> u8 {
            let len = input.len();

            let mut res = {
                // SAFETY: we know for a fact that the supplied input
                // will hold that invariant.
                let num = unsafe { std::str::from_utf8_unchecked(input) };
                u8::from_str_radix(num, 16).unwrap()
            };

            if len == 1 {
                res *= 16;
            }

            res
        }

        alt((map(take(2usize), inner), map(take(1usize), inner)))(input)
    }

    fn parse_hexadecimal_string(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, value) = take_within_balanced(b'<', b'>')(input)?;
        let (d, uvec) = many0(Self::parse_hexadecimal_bigram)(value)?;
        assert!(d.is_empty());
        Ok((input, Self::HexString(uvec)))
    }

    fn parse_name_string(input: &[u8]) -> IResult<&[u8], String> {
        fn escaped_char(input: &[u8]) -> IResult<&[u8], Option<char>> {
            let (input, _) = take(1usize)(input)?;

            let (input, num) = take(2usize)(input)?;
            let (_, n) = opt(parse_hexadecimal_bigram)(num)?;

            Ok((input, n.map(|n| n as char)))
        }

        let (input, _) = tag(b"/")(input)?;
        let (input, value) = take_till(|b| {
            is_space_or_newline(b) || b == b'/' || b == b'<' || b == b'[' || b == b'('
        })(input)?;
        let (d, lines) = many0(parse_string_with_escapes(b'#', escaped_char))(value)?;
        assert!(d.is_empty());
        Ok((input, lines.join("")))
    }

    fn parse_name(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, name) = Self::parse_name_string(input)?;
        Ok((input, Self::Name(name)))
    }

    fn parse_array(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, value) = take_within_balanced(b'[', b']')(input)?;

        // We need to remove preceding whitespace.
        let (value, _) = take_whitespace(value)?;
        let (r, array) = many0(Self::parse)(value)?;

        assert!(
            r.is_empty(),
            "Remainder is not empty: {:?}",
            String::from_utf8_lossy(r)
        );
        Ok((input, Self::Array(array)))
    }

    fn parse_dictionary_raw(input: &[u8]) -> IResult<&[u8], HashMap<String, Self>> {
        // dictionaries are enclosed by double angle brackets.
        let (input, value) = take_within_balanced(b'<', b'>')(input)?;
        let (d, value) = take_within_balanced(b'<', b'>')(value)?;

        if !d.is_empty() {
            return Err(Err::Error(Error::from_error_kind(
                input,
                ErrorKind::TakeTill1,
            )));
        }

        fn parse_key_value(input: &[u8]) -> IResult<&[u8], (String, Object)> {
            let (input, key) = Object::parse_name_string(input)?;
            let (input, _) = take_whitespace(input)?;
            let (input, obj) = Object::parse(input)?;

            Ok((input, (key, obj)))
        }

        let (value, _) = take_whitespace(value)?;
        let (r, array) = many0(parse_key_value)(value)?;
        assert!(
            r.is_empty(),
            "Remainder is not empty: {:?}",
            String::from_utf8_lossy(r)
        );

        Ok((input, array.into_iter().collect()))
    }

    fn parse_null(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, _) = tag(b"null")(input)?;
        Ok((input, Self::Null))
    }

    fn parse_reference(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, reference) = Reference::parse(input)?;
        let (input, _) = tag(b" R")(input)?;
        Ok((input, Self::Reference(reference)))
    }

    fn parse_stream_body(input: &[u8], length: usize) -> IResult<&[u8], &[u8]> {
        let (input, body) = take(length)(input)?;

        let (input, _) = take_eol(input)?;
        let (input, _) = tag(b"endstream")(input)?;

        Ok((input, body))
    }

    fn parse_stream_or_dict(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, dict) = Self::parse_dictionary_raw(input)?;

        let Ok((input, _)) = delimited(take_whitespace, tag(b"stream"), take_eol_no_r)(input)
        else {
            return Ok((input, Self::Dictionary(dict)));
        };

        let length: usize = dict
            .get("Length")
            .expect("`Length` is a required field in a stream dictionnary.")
            .try_into()
            .expect("`Length` key has to be usize-compatible.");

        // Cut to commit to this branch
        let (input, body) = cut(move |i| Self::parse_stream_body(i, length))(input)?;

        let mut filters = Vec::new();

        match dict.get("Filter") {
            None => {}
            Some(Self::Name(n)) => filters.push(Filter::from_name(n)),
            Some(Self::Array(a)) => {
                for f in a {
                    let Self::Name(n) = f else {
                        unreachable!("Per the specs, it MUST be an array of names.")
                    };
                    filters.push(Filter::from_name(n))
                }
            }
            _ => unreachable!(
                "Per the specs, the `Filter` key must be empty, a name of an array of names."
            ),
        }

        let stream = body.to_owned();

        let stream = Stream { stream, filters };

        Ok((input, Self::Stream(stream)))
    }

    /// Parse a single PDF object.
    ///
    /// Note that PDF objects are *not* delimited by `obj` and `endobj`.
    /// Such a case denotes a reference, which is not the same thing.
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        // Necessary in case we apply many0.
        if input.is_empty() {
            return Err(Err::Error(Error::from_error_kind(
                input,
                ErrorKind::NonEmpty,
            )));
        }

        let (input, obj) = alt((
            Self::parse_stream_or_dict,
            Self::parse_name,
            Self::parse_array,
            Self::parse_null,
            Self::parse_boolean,
            Self::parse_reference,
            Self::parse_literal_string,
            Self::parse_numeric,
            Self::parse_hexadecimal_string,
        ))(input)?;

        let (input, _) = take_whitespace(input)?;

        Ok((input, obj))
    }

    /// Parse an object reference.
    pub fn parse_referenced(input: &[u8]) -> IResult<&[u8], (Option<Reference>, Self)> {
        let (input, reference) = opt(tuple((Reference::parse, take_whitespace1)))(input)?;
        let (input, _) = (tag(b"obj"), take_whitespace1).parse(input)?;
        let (input, obj) = Self::parse(input)?;
        let (input, _) = (tag(b"endobj"), take_whitespace1).parse(input)?;

        Ok((input, (reference.map(|(r, _)| r), obj)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! obj {
        (b:$val:literal) => {
            Object::Boolean($val)
        };
        (i:$val:literal) => {
            Object::Integer($val)
        };
        (r:$val:literal) => {
            Object::Real($val)
        };
        (t:$val:literal) => {
            Object::LiteralString($val.to_string())
        };
        (h:$val:tt) => {
            Object::HexString($val.to_vec())
        };
        (s:$val:literal) => {
            Object::Stream(Stream{stream: $val.to_vec(), filters: Vec::new()})
        };
        (s:$val:literal | $filters:tt) => {
            Object::Stream(Stream{stream: $val.to_vec(), filters: $filters})
        };
        (n:$val:literal) => {
            Object::Name($val.to_string())
        };
        ($($o:expr),+ $(,)?) => {
            Object::Array(vec![$($o),+])
        };
        ($($k:literal $v:expr),+ $(,)?) => {
            Object::Dictionary(vec![$(($k.to_string(), $v)),+].into_iter().collect())
        };
    }

    macro_rules! check_parse {
        ($prev:literal boolean $next:literal) => {
            let (input, obj) = Object::parse_boolean($prev).unwrap();
            assert_eq!(obj, Object::Boolean($next));
            assert!(input.is_empty());
        };
        ($prev:literal integer $next:literal) => {
            let (input, obj) = Object::parse_integer($prev).unwrap();
            assert_eq!(obj, Object::Integer($next));
            assert!(input.is_empty());
        };
        ($prev:literal real $next:literal) => {
            let (input, obj) = Object::parse_real($prev).unwrap();
            assert_eq!(obj, Object::Real($next));
            assert!(input.is_empty());
        };
        ($prev:literal literal_string $next:literal) => {
            let (input, obj) = Object::parse_literal_string($prev).unwrap();
            assert_eq!(obj, Object::LiteralString($next.to_string()));
            assert!(input.is_empty());
        };
        ($prev:literal hex_string $next:tt) => {
            let (input, obj) = Object::parse_hexadecimal_string($prev).unwrap();
            assert_eq!(obj, Object::HexString($next.to_vec()));
            assert!(input.is_empty());
        };
        ($prev:literal name $next:tt) => {
            let (input, obj) = Object::parse_name($prev).unwrap();
            assert_eq!(obj, Object::Name($next.to_string()));
            assert!(input.is_empty());
        };
        ($prev:literal name $next:tt + $rem:literal) => {
            let (input, obj) = Object::parse_name($prev).unwrap();
            assert_eq!(obj, Object::Name($next.to_string()));
            assert_eq!(input, $rem);
        };
        ($prev:literal array $next:expr) => {
            let (input, obj) = Object::parse_array($prev).unwrap();
            assert_eq!(obj, $next);
            assert!(input.is_empty());
        };
        ($prev:literal dict $next:expr) => {
            let (input, obj) = Object::parse_stream_or_dict($prev).unwrap();
            assert_eq!(obj, $next);
            assert!(input.is_empty());
        };
        ($prev:literal any $next:expr) => {
            let (input, obj) = Object::parse($prev).unwrap();
            assert_eq!(obj, $next);
            assert!(input.is_empty());
        };
        ($val:literal stream $next:expr) => {
            let (input, obj) = Object::parse_stream_or_dict($val).unwrap();
            assert_eq!(obj, $next);
            assert!(input.is_empty());
        };
    }

    #[test]
    fn bool() {
        check_parse!(b"true" boolean true);
        check_parse!(b"false" boolean false);
    }

    #[test]
    fn integer() {
        check_parse!(b"123" integer 123);
        check_parse!(b"+123" integer 123);
        check_parse!(b"-123" integer -123);
    }

    #[test]
    fn real() {
        check_parse!(b"123." real 123.0);
        check_parse!(b"+123." real 123.0);
        check_parse!(b"-123.0" real -123.0);
        check_parse!(b"-.1" real -0.1);
    }

    #[test]
    fn literal_string() {
        check_parse!(b"(test)" literal_string "test");
        check_parse!(b"(test\n)" literal_string "test\n");
        check_parse!(b"(test (with inner parenthesis))" literal_string "test (with inner parenthesis)");

        check_parse!(b"(\\0533)" literal_string "+3");

        check_parse!(b"(te\\\\st)" literal_string "te\\st");
        check_parse!(b"(te\\\nst)" literal_string "test");
    }

    #[test]
    fn hex_string() {
        check_parse!(b"<901FA3>" hex_string [144, 31, 163]);
        check_parse!(b"<901FA>" hex_string [144, 31, 160]);
    }

    #[test]
    fn name() {
        // Examples from the ISO specs
        check_parse!(b"/Name1" name "Name1");
        check_parse!(b"/ASomewhatLongerName" name "ASomewhatLongerName");
        check_parse!(b"/A;Name_With-Various***Characters?" name "A;Name_With-Various***Characters?");
        check_parse!(b"/1.2" name "1.2");
        check_parse!(b"/$$" name "$$");
        check_parse!(b"/@pattern" name "@pattern");
        check_parse!(b"/.notdef" name ".notdef");
        check_parse!(b"/Lime#20Green\n" name "Lime Green" + b"\n");
        check_parse!(b"/paired#28#29parentheses(" name "paired()parentheses" + b"(");
        check_parse!(b"/The_Key_of_F#23_Minor" name "The_Key_of_F#_Minor");
        check_parse!(b"/A#42" name "AB");
    }

    #[test]
    fn array() {
        check_parse!(b"[1 true /Test]" array obj![obj!(i:1), obj!(b:true), obj!(n:"Test")]);
        check_parse!(b"[1 (true) /Test]" array obj![obj!(i:1), obj!(t:"true"), obj!(n:"Test")]);
    }

    #[test]
    fn dictionary() {
        let d = obj![
            "Type" obj!(n:"Example"),
            "Version" obj!(r:0.01),
            "Int" obj!(i:12),
            "String" obj!(t:"a string"),
            "Subdict" obj!["Key" obj!(t:"value")]
        ];

        check_parse!(b"<</Type /Example /Version 0.01 /Int 12 /String (a string) /Subdict <</Key (value)>>>>" dict d);
    }

    #[test]
    fn stream() {
        check_parse!(b"<</Length 9>>\nstream\n123456789\nendstream" stream obj!(s:b"123456789"));
    }

    #[test]
    fn any() {
        check_parse!(b"123.   " any Object::Real(123.0));
        check_parse!(b"false" any Object::Boolean(false));
        check_parse!(b"true " any Object::Boolean(true));
        // check_parse!(b"-123\n" any Object::Integer(-123));
        check_parse!(b"[<2B><3>]" any obj![obj!(h:[43]), obj!(h:[48])]);
        check_parse!(b"(-123)\n" any Object::LiteralString("-123".to_string()));
        check_parse!(b"<901FA>" any Object::HexString(vec![144, 31, 160]));
        check_parse!(b"/The_Key_of_F#23_Minor" any Object::Name("The_Key_of_F#_Minor".to_string()));
        check_parse!(b"1 0 R" any Object::Reference(Reference { obj: 1, gen: 0 }));
        check_parse!(b"null" any Object::Null);
    }

    mod object {
        use super::super::*;

        #[test]
        fn parse_full_bool() {
            let (input, (_, obj)) = Object::parse_referenced(b"obj\ntrue  \nendobj\n").unwrap();
            assert_eq!(obj, Object::Boolean(true));
            assert!(input.is_empty());
        }
    }
}
