use std::num::TryFromIntError;

use nom::{combinator::opt, sequence::pair, IResult};

use crate::utilities::{parse_digits, parse_sign};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Integer(pub i32);

impl Integer {
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, (sign, mut num)) = pair(opt(parse_sign), parse_digits::<i32, _>)(input)?;

        if let Some(b"-") = sign {
            num = -num;
        }

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
