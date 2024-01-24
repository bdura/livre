use nom::{
    branch::alt,
    bytes::complete::{tag, take_till, take_while},
    character::complete::{digit0, digit1},
    combinator::{opt, recognize},
    sequence::{pair, separated_pair, tuple, Tuple},
    IResult,
};

use super::utilities::{take_whitespace, take_whitespace1, take_within_balanced};

#[derive(Debug, Clone, PartialEq)]
pub enum Object {
    Boolean(bool),
    Integer(i32),
    Real(f32),
    LiteralString(String),
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
        let (input, value) = take_within_balanced(b'(', b')')(input)?;

        let s = String::from_utf8(value.to_vec()).unwrap();

        Ok((input, Self::LiteralString(s)))
    }

    fn parse_any(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, obj) = alt((
            Self::parse_boolean,
            Self::parse_numeric,
            Self::parse_literal_string,
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
        check_parse!(b"(test (with inner parenthesis))" literal_string "test (with inner parenthesis)");
    }

    #[test]
    fn any() {
        check_parse!(b"123.   " any Object::Real(123.0));
        check_parse!(b"false" any Object::Boolean(false));
        check_parse!(b"true " any Object::Boolean(true));
        check_parse!(b"-123\n" any Object::Integer(-123));
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
