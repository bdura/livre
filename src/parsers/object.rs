use nom::{
    branch::alt,
    bytes::complete::{tag, take, take_till, take_while},
    character::complete::{digit0, digit1},
    combinator::{map, opt, recognize},
    error::{Error, ErrorKind, ParseError},
    multi::many0,
    sequence::{pair, separated_pair, Tuple},
    Err, IResult,
};

use crate::parsers::utilities::{parse_hexadecimal_bigram, parse_string_with_escapes};

use super::utilities::{parse_octal, take_whitespace, take_whitespace1, take_within_balanced};

#[derive(Debug, Clone, PartialEq)]
pub enum Object {
    Boolean(bool),
    Integer(i32),
    Real(f32),
    LiteralString(String),
    HexString(Vec<u8>),
    Name(String),
    Array(Vec<Object>),
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
        let (d, array) = many0(Self::parse_any)(value)?;
        assert!(d.is_empty());
        Ok((input, Self::Array(array)))
    }

    fn parse_any(input: &[u8]) -> IResult<&[u8], Self> {
        // Necessary in case we apply many0.
        if input.is_empty() {
            return Err(Err::Error(Error::from_error_kind(
                input,
                ErrorKind::NonEmpty,
            )));
        }

        let (input, obj) = alt((
            Self::parse_boolean,
            Self::parse_numeric,
            Self::parse_literal_string,
            Self::parse_hexadecimal_string,
            Self::parse_name,
            Self::parse_array,
        ))(input)?;

        let (input, _) = take_whitespace(input)?;

        Ok((input, obj))
    }

    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, _) = (tag(b"obj"), take_whitespace1).parse(input)?;
        let (input, obj) = Self::parse_any(input)?;
        let (input, _) = (tag(b"endobj"), take_whitespace1).parse(input)?;

        Ok((input, obj))
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
        (s:$val:literal) => {
            Object::LiteralString($val.to_string())
        };
        (n:$val:literal) => {
            Object::Name($val.to_string())
        };
        ($($o:expr),+ $(,)?) => {
            Object::Array(vec![$($o),+])
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
        ($prev:literal any $next:expr) => {
            let (input, obj) = Object::parse_any($prev).unwrap();
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
        check_parse!(b"[1 (true) /Test]" array obj![obj!(i:1), obj!(s:"true"), obj!(n:"Test")]);
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
