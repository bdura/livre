use std::{
    num::NonZeroU8,
    ops::{AddAssign, MulAssign, Neg},
};

use winnow::{
    ascii::{digit1, float},
    combinator::{alt, opt, preceded, trace},
    BStr, PResult, Parser,
};

use crate::Extract;

/// Convert a slice of bytes representing decimal integers to a Rust number
///
/// WARNING: this function expects a slice of digits.
fn convert_unsigned<T>(digits: &[u8]) -> T
where
    T: From<u8> + MulAssign<T> + AddAssign<T>,
{
    // Convert each byte to T
    let mut digits = digits.into_iter().map(|byte| T::from(byte - b'0'));

    let mut result = digits
        .next()
        .expect("digit1 parser returns at least 1 digit");

    for digit in digits {
        result *= T::from(10);
        result += digit;
    }

    result
}

/// Convert a slice of bytes representing signed decimal integers to a Rust number.
///
/// We need a separate function from [`convert_unsigned`], because a signed integer
/// cannot be converted from a `u8`.
///
/// WARNING: this function expects a slice of digits.
///
/// NOTE: since `i<bits>::MIN = - (i<bits>::MAX + 1)`, the output of this function
/// is the negative associated number. It should be converted to a positive integer
/// if need be.
fn convert_negative_signed<T>(digits: &[u8]) -> T
where
    T: From<i8> + MulAssign<T> + AddAssign<T>,
{
    // Convert each byte to T
    let mut digits = digits
        .into_iter()
        .copied()
        .map(|byte| -((byte - b'0') as i8))
        .map(T::from);

    let mut result = digits
        .next()
        .expect("digit1 parser returns at least 1 digit");

    for digit in digits {
        result *= T::from(10);
        result += digit;
    }

    result
}

fn parse_unsigned<T>(input: &mut &BStr) -> PResult<T>
where
    T: From<u8> + MulAssign<T> + AddAssign<T>,
{
    trace(
        "livre-unsigned",
        preceded(opt(b'+'), digit1.map(convert_unsigned)),
    )
    .parse_next(input)
}

fn is_neg(input: &mut &BStr) -> PResult<bool> {
    trace(
        "livre-is-neg",
        opt(alt((b'-', b'+'))).map(|s| s == Some(b'-')),
    )
    .parse_next(input)
}

fn parse_signed<T>(input: &mut &BStr) -> PResult<T>
where
    T: From<i8> + MulAssign<T> + AddAssign<T> + Neg<Output = T>,
{
    fn inner<T>(input: &mut &BStr) -> PResult<T>
    where
        T: From<i8> + MulAssign<T> + AddAssign<T> + Neg<Output = T>,
    {
        let neg = is_neg(input)?;
        let mut num: T = digit1.map(convert_negative_signed).parse_next(input)?;

        if !neg {
            num = -num;
        }

        Ok(num)
    }

    trace("livre-signed", inner).parse_next(input)
}

macro_rules! unsigned {
    ($($name:ident)+) => {
        $(
            impl Extract<'_> for $name {
                fn extract(input: &mut &BStr) -> PResult<Self> {
                    parse_unsigned(input)
                }
            }
        )+
    };
}

macro_rules! signed {
    ($($name:ident)+) => {
        $(
            impl Extract<'_> for $name {
                fn extract(input: &mut &BStr) -> PResult<Self> {
                    parse_signed(input)
                }
            }
        )+
    };
}

macro_rules! real {
    ($($name:ident)+) => {
        $(
            impl Extract<'_> for $name {
                fn extract(input: &mut &BStr) -> PResult<Self> {
                    float(input)
                }
            }
        )+
    };
}

unsigned! {
    u8
    u16
    u32
    u64
    u128
    usize
}

signed! {
    i8
    i16
    i32
    i64
    i128
    isize
}

real! {
    f32
    f64
}

impl Extract<'_> for NonZeroU8 {
    fn extract(input: &mut &BStr) -> PResult<Self> {
        let n = u8::extract.verify(|&r| r != 0).parse_next(input)?;

        // SAFETY: the verification happens at the parser level ↑
        let n = unsafe { NonZeroU8::new_unchecked(n) };

        Ok(n)
    }
}

#[cfg(test)]
mod tests {

    use std::{fmt::Debug, i16, u16};

    use rstest::rstest;

    use crate::{extraction::extract, Extract};

    #[rstest]
    #[case(b"42", 42u8)]
    #[case(b"+42", 42u8)]
    #[case(b"42", 42usize)]
    #[case(b"42", 42u16)]
    #[case(b"65535", u16::MAX)]
    #[case(b"32767", i16::MAX)]
    #[case(b"-32768", i16::MIN)]
    #[case(b"42", 42i16)]
    #[case(b"-42", -42i16)]
    #[case(b"42", 42.0f32)]
    #[case(b"42", 42.0f64)]
    #[case(b"00042", 42.0f64)]
    #[case(b"-0.42", -0.42f64)]
    fn extraction<T>(#[case] input: &[u8], #[case] expected: T)
    where
        T: for<'a> Extract<'a> + Debug + PartialEq,
    {
        let res = extract(&mut input.as_ref()).unwrap();
        assert_eq!(expected, res);
    }
}
