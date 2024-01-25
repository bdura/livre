use std::collections::HashMap;

use nom::{
    branch::alt,
    bytes::complete::{tag, take, take_till, take_until, take_while},
    character::complete::{digit0, digit1},
    combinator::{map, opt, recognize},
    error::{Error, ErrorKind, ParseError},
    multi::many0,
    sequence::{pair, separated_pair, terminated, tuple, Tuple},
    Err, IResult,
};

use super::utilities::{
    parse_octal, take_eol, take_whitespace, take_whitespace1, take_within_balanced,
};
use crate::parsers::utilities::{parse_hexadecimal_bigram, parse_string_with_escapes};

#[derive(Debug, Clone, PartialEq)]
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
    Stream(Vec<u8>),
    Reference(Reference),
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

// #[derive(Debug, Clone, PartialEq, Default)]
// pub struct StreamConfig {
//     length: usize,
//     filter: Option<Vec<String>>,
//     decode_params: Option<Vec<HashMap<String, String>>>,
// }

// #[derive(Debug, Clone, PartialEq)]
// pub struct Stream {
//     config: StreamConfig,
//     stream: Vec<u8>,
// }

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
    /// This is needed otherwise all numbers are interpreted as integers,
    /// discarding digits after the decimal point.
    fn parse_numeric(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, value) = take_while(|c| c != b' ')(input)?;

        if value.contains(&b'.') {
            let (_, obj) = Object::parse_real(value)?;
            Ok((input, obj))
        } else {
            let (_, obj) = Object::parse_integer(value)?;
            Ok((input, obj))
        }
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

    fn parse_name(input: &[u8]) -> IResult<&[u8], Self> {
        fn escaped_char(input: &[u8]) -> IResult<&[u8], Option<char>> {
            let (input, _) = take(1usize)(input)?;

            let (input, num) = take(2usize)(input)?;
            let (_, n) = opt(parse_hexadecimal_bigram)(num)?;

            Ok((input, n.map(|n| n as char)))
        }

        let (input, _) = tag(b"/")(input)?;
        let (input, value) = take_till(|b| b == b' ' || b == b'/')(input)?;
        let (d, lines) = many0(parse_string_with_escapes(b'#', escaped_char))(value)?;
        assert!(d.is_empty());
        Ok((input, Self::Name(lines.join(""))))
    }

    fn parse_array(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, value) = take_within_balanced(b'[', b']')(input)?;
        let (d, array) = many0(Self::parse_any_object)(value)?;
        assert!(d.is_empty());
        Ok((input, Self::Array(array)))
    }

    fn parse_dictionary(input: &[u8]) -> IResult<&[u8], Self> {
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
            let (input, key_obj) = Object::parse_name(input)?;
            let key = if let Object::Name(key) = key_obj {
                key
            } else {
                return Err(Err::Error(Error::from_error_kind(input, ErrorKind::IsNot)));
            };

            let (input, _) = take_whitespace1(input)?;

            let (input, obj) = Object::parse_any_object(input)?;

            Ok((input, (key, obj)))
        }

        let (d, array) = many0(parse_key_value)(value)?;
        assert!(d.is_empty());

        let res = Object::Dictionary(array.into_iter().collect());

        Ok((input, res))
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

    fn parse_any_object(input: &[u8]) -> IResult<&[u8], Self> {
        // Necessary in case we apply many0.
        if input.is_empty() {
            return Err(Err::Error(Error::from_error_kind(
                input,
                ErrorKind::NonEmpty,
            )));
        }

        let (input, obj) = alt((
            Self::parse_null,
            Self::parse_boolean,
            Self::parse_reference,
            Self::parse_numeric,
            Self::parse_literal_string,
            Self::parse_name,
            Self::parse_array,
            Self::parse_dictionary,
            Self::parse_hexadecimal_string,
        ))(input)?;

        let (input, _) = take_whitespace(input)?;

        Ok((input, obj))
    }

    pub fn parse_stream(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, _dict) = Self::parse_dictionary(input)?;
        let (input, _) = take_whitespace(input)?;
        let (input, _) = (tag(b"stream"), take_eol).parse(input)?;
        let (input, stream) =
            terminated(take_until(b"\nendstream".as_slice()), tag(b"\nendstream"))(input)?;

        // let stream = Stream {
        //     config: Default::default(),
        //     stream: stream.to_owned(),
        // };

        Ok((input, Self::Stream(stream.to_owned())))
    }

    pub fn parse_object(input: &[u8]) -> IResult<&[u8], (Option<Reference>, Self)> {
        let (input, reference) = opt(tuple((Reference::parse, take_whitespace1)))(input)?;
        let (input, _) = (tag(b"obj"), take_whitespace1).parse(input)?;
        let (input, obj) = Self::parse_any_object(input)?;
        let (input, _) = (tag(b"endobj"), take_whitespace1).parse(input)?;

        Ok((input, (reference.map(|(r, _)| r), obj)))
    }

    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        alt((map(Self::parse_object, |(_, o)| o), Self::parse_stream))(input)
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
        (h:$val:literal) => {
            Object::HexString($val.to_vec())
        };
        (s:$val:literal) => {
            Object::Stream($val.to_vec())
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
        ($prev:literal array $next:expr) => {
            let (input, obj) = Object::parse_array($prev).unwrap();
            assert_eq!(obj, $next);
            assert!(input.is_empty());
        };
        ($prev:literal dict $next:expr) => {
            let (input, obj) = Object::parse_dictionary($prev).unwrap();
            assert_eq!(obj, $next);
            assert!(input.is_empty());
        };
        ($prev:literal any $next:expr) => {
            let (input, obj) = Object::parse_any_object($prev).unwrap();
            assert_eq!(obj, $next);
            assert!(input.is_empty());
        };
        ($val:literal stream $next:expr) => {
            let (input, obj) = Object::parse_stream($val).unwrap();
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
        check_parse!(b"/Lime#20Green" name "Lime Green");
        check_parse!(b"/paired#28#29parentheses" name "paired()parentheses");
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
        check_parse!(b"-123\n" any Object::Integer(-123));
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
            let (input, obj) = Object::parse(b"obj\ntrue  \nendobj\n").unwrap();
            assert_eq!(obj, Object::Boolean(true));
            assert!(input.is_empty());
        }
    }
}
