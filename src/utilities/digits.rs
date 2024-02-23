use std::{fmt::Debug, str::FromStr};

use nom::{
    branch::alt,
    bytes::complete::{tag, take},
    character::complete::{digit1, oct_digit1},
    combinator::map,
    IResult,
};

/// Parse up to 3 bytes to get the number represented by the underlying octal code.
pub fn parse_octal(input: &[u8]) -> IResult<&[u8], u8> {
    let value = &input[..input.len().min(3)];
    let (_, num) = oct_digit1(value)?;

    let input = &input[num.len()..];

    let s = unsafe { std::str::from_utf8_unchecked(num) };
    let n = u8::from_str_radix(s, 8).expect("We know it's a valid number.");

    Ok((input, n))
}

pub fn parse_hexadecimal_bigram(input: &[u8]) -> IResult<&[u8], u8> {
    fn inner(input: &[u8]) -> u8 {
        let len = input.len();

        let mut res = {
            // TODO: check the invariant.
            let num = std::str::from_utf8(input).unwrap();
            u8::from_str_radix(num, 16).unwrap()
        };

        if len == 1 {
            res *= 16;
        }

        res
    }

    alt((map(take(2usize), inner), map(take(1usize), inner)))(input)
}

pub fn parse_digits<O, E>(input: &[u8]) -> IResult<&[u8], O>
where
    O: FromStr<Err = E>,
    E: Debug,
{
    let (input, digits) = digit1(input)?;

    // SAFETY: we know for a fact that `digits` contains digits only,
    // and are therefore both utf-8-encoded and parsable.
    let n = unsafe { std::str::from_utf8_unchecked(digits).parse().unwrap() };

    Ok((input, n))
}

pub fn parse_sign(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((tag(b"+"), tag(b"-")))(input)
}
