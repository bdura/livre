use std::num::TryFromIntError;

use nom::{
    character::complete::digit1,
    combinator::{opt, recognize},
    sequence::pair,
    IResult,
};

use crate::utilities::parse_sign;

/// Represents a boolean within a PDF.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Integer(pub(crate) i32);

impl Integer {
    pub(crate) fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, num) = recognize(pair(opt(parse_sign), digit1))(input)?;

        // SAFETY: we know for a fact that `num` only includes ascii characters
        let num_str = unsafe { std::str::from_utf8_unchecked(num) };

        let num = num_str
            .parse()
            .expect("[+-]?\\d+ is parseable as an integer.");

        Ok((input, Self(num)))
    }
}

impl From<Integer> for i32 {
    fn from(value: Integer) -> Self {
        value.0
    }
}

impl From<i32> for Integer {
    fn from(value: i32) -> Self {
        Self(value)
    }
}

macro_rules! into {
    ($into:ty) => {
        impl From<Integer> for $into {
            fn from(Integer(value): Integer) -> Self {
                value.into()
            }
        }
    };
    (try $into:ty) => {
        impl TryFrom<Integer> for $into {
            type Error = TryFromIntError;

            fn try_from(Integer(value): Integer) -> Result<Self, Self::Error> {
                value.try_into()
            }
        }
    };
}

into!(try i8);
into!(try i16);
into!(i64);
into!(try u8);
into!(try u16);
into!(try u32);
into!(try u64);
into!(try usize);

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn parse(input: &[u8]) -> Integer {
        let (_, obj) = Integer::parse(input).unwrap();
        obj
    }

    #[allow(clippy::bool_assert_comparison)]
    #[rstest]
    #[case(b"1", 1)]
    #[case(b"0", 0)]
    #[case(b"-0", 0)]
    #[case(b"-10", -10)]
    fn test_parse(#[case] input: &[u8], #[case] result: i32) {
        assert_eq!(parse(input), result.into());
        assert_eq!(result, parse(input).into());
    }
}
