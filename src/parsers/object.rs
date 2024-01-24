use nom::{
    branch::alt, bytes::complete::tag, character::complete::digit1, combinator::opt,
    sequence::Tuple, IResult,
};

use super::utilities::take_whitespace1;

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
        let (input, _) = take_whitespace1(input)?;

        let obj = match b {
            b"true" => Self::Boolean(true),
            b"false" => Self::Boolean(false),
            _ => unreachable!("The tags should only match true or false."),
        };

        Ok((input, obj))
    }

    fn parse_integer(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, sign) = opt(alt((tag(b"+"), tag(b"-"))))(input)?;
        let (input, num) = digit1(input)?;
        let (input, _) = take_whitespace1(input)?;

        // SAFETY: we know for a fact that `num` only includes digits
        let num = unsafe { String::from_utf8_unchecked(num.to_vec()) };

        let mut num: i32 = num.parse().unwrap();

        if let Some(b"-") = sign {
            num = -num;
        }

        Ok((input, Self::Integer(num)))
    }

    fn parse_real(_input: &[u8]) -> IResult<&[u8], Self> {
        todo!()
    }

    fn parse_literal_string(_input: &[u8]) -> IResult<&[u8], Self> {
        todo!()
    }

    fn parse_any(input: &[u8]) -> IResult<&[u8], Self> {
        alt((
            Self::parse_boolean,
            Self::parse_integer,
            Self::parse_real,
            Self::parse_literal_string,
        ))(input)
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
    mod boolean {
        use super::super::*;

        #[test]
        fn parse_true() {
            let (input, boolean) = Object::parse_boolean(b"true ").unwrap();
            assert_eq!(boolean, Object::Boolean(true));
            assert!(input.is_empty());
        }

        #[test]
        fn parse_false() {
            let (input, boolean) = Object::parse_boolean(b"false\n").unwrap();
            assert_eq!(boolean, Object::Boolean(false));
            assert!(input.is_empty());
        }

        #[test]
        fn parse_false_and_whitespaces() {
            let (input, boolean) = Object::parse_boolean(b"false\n    \n\n").unwrap();
            assert_eq!(boolean, Object::Boolean(false));
            assert!(input.is_empty());
        }
    }

    mod integer {
        use super::super::*;

        #[test]
        fn parse_positive() {
            let (input, integer) = Object::parse_integer(b"123 ").unwrap();
            assert_eq!(integer, Object::Integer(123));
            assert!(input.is_empty());
        }

        #[test]
        fn parse_signed_positive() {
            let (input, integer) = Object::parse_integer(b"+123 ").unwrap();
            assert_eq!(integer, Object::Integer(123));
            assert!(input.is_empty());
        }

        #[test]
        fn parse_negative() {
            let (input, integer) = Object::parse_integer(b"-123 ").unwrap();
            assert_eq!(integer, Object::Integer(-123));
            assert!(input.is_empty());
        }
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
