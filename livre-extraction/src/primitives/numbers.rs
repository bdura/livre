use nom::IResult;

use livre_utilities::{parse_integer, parse_number, parse_unsigned_integer};

use crate::extraction::Extract;

macro_rules! unsigned {
    ($type:ty) => {
        impl Extract<'_> for $type {
            fn extract(input: &[u8]) -> IResult<&[u8], Self> {
                parse_unsigned_integer(input)
            }
        }
    };
}

macro_rules! signed {
    ($type:ty) => {
        impl Extract<'_> for $type {
            fn extract(input: &[u8]) -> IResult<&[u8], Self> {
                parse_integer(input)
            }
        }
    };
}

macro_rules! real {
    ($type:ty) => {
        impl Extract<'_> for $type {
            fn extract(input: &'_ [u8]) -> IResult<&'_ [u8], Self> {
                parse_number(input)
            }
        }
    };
}

unsigned!(u8);
unsigned!(u16);
unsigned!(u32);
unsigned!(u64);
unsigned!(usize);
unsigned!(u128);

signed!(i8);
signed!(i16);
signed!(i32);
signed!(i64);
signed!(isize);
signed!(i128);

real!(f32);
real!(f64);

#[cfg(test)]
mod tests {
    use crate::parse;
    use rstest::rstest;

    #[rstest]
    #[case(b"1", 1)]
    #[case(b"+1", 1)]
    #[case(b"0", 0)]
    #[case(b"+0", 0)]
    fn unsigned_int(#[case] input: &[u8], #[case] result: u128) {
        assert_eq!(result, parse(input).unwrap());

        let r = result as u64;
        assert_eq!(r, parse(input).unwrap());

        let r = result as usize;
        assert_eq!(r, parse(input).unwrap());

        let r = result as u32;
        assert_eq!(r, parse(input).unwrap());

        let r = result as u16;
        assert_eq!(r, parse(input).unwrap());

        let r = result as u8;
        assert_eq!(r, parse(input).unwrap());
    }

    #[rstest]
    #[case(b"1", 1)]
    #[case(b"0", 0)]
    #[case(b"+10", 10)]
    #[case(b"-0", 0)]
    #[case(b"-10", -10)]
    #[case(b"-100", -100)]
    fn signed_int(#[case] input: &[u8], #[case] result: i128) {
        assert_eq!(result, parse(input).unwrap());

        let r = result as i64;
        assert_eq!(r, parse(input).unwrap());

        let r = result as isize;
        assert_eq!(r, parse(input).unwrap());

        let r = result as i32;
        assert_eq!(r, parse(input).unwrap());

        let r = result as i16;
        assert_eq!(r, parse(input).unwrap());

        let r = result as i8;
        assert_eq!(r, parse(input).unwrap());
    }

    #[rstest]
    #[case(b"0", 0)]
    #[case(b"1", 1)]
    #[case(b"10", 10)]
    #[case(b"255", 255)]
    fn parse_u8(#[case] input: &[u8], #[case] result: u8) {
        assert_eq!(result, parse(input).unwrap())
    }

    #[rstest]
    #[case(b"0", 0)]
    #[case(b"+1", 1)]
    #[case(b"10", 10)]
    #[case(b"255", 255)]
    #[case(b"4294967295", u32::MAX)]
    fn parse_u32(#[case] input: &[u8], #[case] result: u32) {
        assert_eq!(result, parse(input).unwrap())
    }

    #[rstest]
    #[case(b"0", 0)]
    #[case(b"1", 1)]
    #[case(b"10", 10)]
    #[case(b"-255", -255)]
    #[case(b"-2147483648", i32::MIN)]
    fn parse_i32(#[case] input: &[u8], #[case] result: i32) {
        assert_eq!(result, parse(input).unwrap())
    }

    #[rstest]
    #[case(b"1.", 1.0)]
    #[case(b"+1.", 1.0)]
    #[case(b"-1.", -1.0)]
    #[case(b"-.1", -0.1)]
    #[case(b".0", 0.0)]
    #[case(b"+0.", 0.0)]
    #[case(b"12345.6789", 12345.6789)]
    #[case(b"-12345.6789", -12345.6789)]
    #[case(b"-7(n)", -7.0)]
    fn reals(#[case] input: &[u8], #[case] result: f64) {
        assert_eq!(result, parse(input).unwrap());

        let r = result as f32;
        assert_eq!(r, parse(input).unwrap());
    }
}
