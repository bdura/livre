use nom::{
    character::complete::digit1,
    combinator::{opt, recognize},
    sequence::tuple,
    IResult,
};

use crate::utilities::parse_sign;

use super::Extract;

// TODO: handle parsing error.
macro_rules! extract {
    ($type:ty) => {
        impl Extract for $type {
            fn extract(input: &[u8]) -> IResult<&[u8], Self> {
                let (input, num) = recognize(tuple((opt(parse_sign), digit1)))(input)?;

                // SAFETY: num is an optional sign, followed by digits.
                let num = unsafe { std::str::from_utf8_unchecked(num) };

                let n = num.parse().unwrap();

                Ok((input, n))
            }
        }
    };
}

extract!(u8);
extract!(i8);
extract!(u16);
extract!(i16);
extract!(u32);
extract!(i32);
extract!(u64);
extract!(i64);
extract!(usize);
extract!(isize);
extract!(u128);
extract!(i128);

#[cfg(test)]
mod tests {
    use crate::extraction::Parse;
    use rstest::rstest;

    #[rstest]
    #[case(b"1", 1)]
    #[case(b"+1", 1)]
    #[case(b"0", 0)]
    #[case(b"+0", 0)]
    fn parse_unsigned(#[case] input: &[u8], #[case] result: u128) {
        assert_eq!(result, input.parse().unwrap());

        let r = result as u64;
        assert_eq!(r, input.parse().unwrap());

        let r = result as usize;
        assert_eq!(r, input.parse().unwrap());

        let r = result as u32;
        assert_eq!(r, input.parse().unwrap());

        let r = result as u16;
        assert_eq!(r, input.parse().unwrap());

        let r = result as u8;
        assert_eq!(r, input.parse().unwrap());
    }

    #[rstest]
    #[case(b"1", 1)]
    #[case(b"0", 0)]
    #[case(b"+10", 10)]
    #[case(b"-0", 0)]
    #[case(b"-10", -10)]
    #[case(b"-100", -100)]
    fn parse_signed(#[case] input: &[u8], #[case] result: i128) {
        assert_eq!(result, input.parse().unwrap());

        let r = result as i64;
        assert_eq!(r, input.parse().unwrap());

        let r = result as isize;
        assert_eq!(r, input.parse().unwrap());

        let r = result as i32;
        assert_eq!(r, input.parse().unwrap());

        let r = result as i16;
        assert_eq!(r, input.parse().unwrap());

        let r = result as i8;
        assert_eq!(r, input.parse().unwrap());
    }

    #[rstest]
    #[case(b"0", 0)]
    #[case(b"1", 1)]
    #[case(b"10", 10)]
    #[case(b"255", 255)]
    fn parse_u8(#[case] input: &[u8], #[case] result: u8) {
        assert_eq!(result, input.parse().unwrap())
    }

    #[rstest]
    #[case(b"0", 0)]
    #[case(b"+1", 1)]
    #[case(b"10", 10)]
    #[case(b"255", 255)]
    #[case(b"4294967295", u32::MAX)]
    fn parse_u32(#[case] input: &[u8], #[case] result: u32) {
        assert_eq!(result, input.parse().unwrap())
    }

    #[rstest]
    #[case(b"0", 0)]
    #[case(b"1", 1)]
    #[case(b"10", 10)]
    #[case(b"-255", -255)]
    #[case(b"-2147483648", i32::MIN)]
    fn parse_i32(#[case] input: &[u8], #[case] result: i32) {
        assert_eq!(result, input.parse().unwrap())
    }
}
