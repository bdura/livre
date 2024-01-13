use nom::{
    branch::alt,
    bytes::complete::{tag, take_while, take_while1},
    character::{is_newline, is_space},
    IResult,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Object {
    Boolean(bool),
    Integer(i32),
    Real(f32),
    LiteralString(String),
}

/// Consumes the whitespace (at least one)
fn take_whitespace1(input: &[u8]) -> IResult<&[u8], &[u8]> {
    take_while1(|v| is_space(v) || is_newline(v))(input)
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
}
