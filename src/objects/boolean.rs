use nom::{branch::alt, bytes::complete::tag, IResult};

/// Represents a boolean within a PDF.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Boolean(pub bool);

impl Boolean {
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, b) = alt((tag("true"), tag("false")))(input)?;

        let obj = match b {
            b"true" => Self(true),
            b"false" => Self(false),
            _ => unreachable!("The tags should only match true or false."),
        };

        Ok((input, obj))
    }
}

impl From<Boolean> for bool {
    fn from(value: Boolean) -> Self {
        value.0
    }
}

impl From<bool> for Boolean {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &[u8]) -> Boolean {
        let (_, obj) = Boolean::parse(input).unwrap();
        obj
    }

    #[allow(clippy::bool_assert_comparison)]
    #[test]
    fn test_parse() {
        assert_eq!(parse(b"true"), Boolean::from(true));
        assert_eq!(false, parse(b"false").into());
    }
}
