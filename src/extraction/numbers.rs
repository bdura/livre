use nom::IResult;

use crate::utilities::{parse_integer, parse_real};

use super::Extract;

// TODO: handle parsing error.
macro_rules! extract_integer {
    ($type:ty) => {
        impl Extract for $type {
            fn extract(input: &[u8]) -> IResult<&[u8], Self> {
                parse_integer(input)
            }
        }
    };
}

// TODO: handle parsing error.
macro_rules! extract_real {
    ($type:ty) => {
        impl Extract for $type {
            fn extract(input: &[u8]) -> IResult<&[u8], Self> {
                parse_real(input)
            }
        }
    };
}

extract_integer!(u8);
extract_integer!(i8);
extract_integer!(u16);
extract_integer!(i16);
extract_integer!(u32);
extract_integer!(i32);
extract_integer!(u64);
extract_integer!(i64);
extract_integer!(usize);
extract_integer!(isize);
extract_integer!(u128);
extract_integer!(i128);

extract_real!(f32);
extract_real!(f64);

#[cfg(test)]
mod tests {
    use crate::extraction::Parse;
    use rstest::rstest;

    #[rstest]
    #[case(b"1", 1)]
    #[case(b"+1", 1)]
    #[case(b"0", 0)]
    #[case(b"+0", 0)]
    fn unsigned_int(#[case] input: &[u8], #[case] result: u128) {
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
    fn signed_int(#[case] input: &[u8], #[case] result: i128) {
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

    #[rstest]
    #[case(b"1.", 1.0)]
    #[case(b"+1.", 1.0)]
    #[case(b"-1.", -1.0)]
    #[case(b"-.1", -0.1)]
    #[case(b".0", 0.0)]
    #[case(b"+0.", 0.0)]
    #[case(b"12345.6789", 12345.6789)]
    #[case(b"-12345.6789", -12345.6789)]
    fn reals(#[case] input: &[u8], #[case] result: f64) {
        assert_eq!(result, input.parse().unwrap());

        let r = result as f32;
        assert_eq!(r, input.parse().unwrap());
    }
}
