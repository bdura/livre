use std::{fmt::Debug, str::FromStr};

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{digit0, digit1, hex_digit1, oct_digit1},
    combinator::{opt, recognize},
    error::{Error, ErrorKind, ParseError},
    sequence::separated_pair,
    Err, IResult,
};

/// Parse up to 3 bytes to get the number represented by the underlying octal code.
pub fn parse_octal(input: &[u8]) -> IResult<&[u8], u8> {
    let value = &input[..input.len().min(3)];
    let (_, num) = oct_digit1(value)?;

    let len = num.len();

    if len == 0 {
        return Err(Err::Error(Error::from_error_kind(
            input,
            ErrorKind::OctDigit,
        )));
    }

    let input = &input[len..];

    // SAFETY: `num` only contains octal digits,
    // and is therefore both utf8-encoded and parseable
    let s = unsafe { std::str::from_utf8_unchecked(num) };
    let n = u8::from_str_radix(s, 8).expect("We know it's a valid number.");

    Ok((input, n))
}

/// Parse up to two bytes to get the number represented by the hexadecimal code.
pub fn parse_hexadecimal_bigram(input: &[u8]) -> IResult<&[u8], u8> {
    // TODO: when we go "streaming"
    // let n = input.len();
    // if n < 2 {
    //     // SAFETY: n < 2, therefore 2 - n is non-zero.
    //     let needed = unsafe { NonZeroUsize::new_unchecked(2 - n) };
    //     return Err(nom::Err::Incomplete(nom::Needed::Size(needed)));
    // }

    let value = &input[..input.len().min(2)];
    let (_, num) = hex_digit1(value)?;

    let len = num.len();

    if len == 0 {
        return Err(Err::Error(Error::from_error_kind(
            input,
            ErrorKind::HexDigit,
        )));
    }

    let input = &input[len..];

    // SAFETY: `num` only contains hex digits,
    // and is therefore both utf8-encoded and parseable
    let s = unsafe { std::str::from_utf8_unchecked(num) };
    let mut n = u8::from_str_radix(s, 16).expect("We know it's a valid number.");

    if len == 1 {
        n *= 16;
    }

    Ok((input, n))
}

pub fn parse_digits<O, E>(input: &[u8]) -> IResult<&[u8], O>
where
    O: FromStr<Err = E>,
    E: Debug,
{
    let (input, digits) = digit1(input)?;

    // SAFETY: we know for a fact that `digits` contains digits only,
    // and are therefore both utf-8-encoded and parsable.
    let num = unsafe { std::str::from_utf8_unchecked(digits) };
    let n = num.parse().unwrap();

    Ok((input, n))
}

pub fn parse_sign(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((tag("+"), tag("-")))(input)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(b"001", 1)]
    #[case(b"010", 8)]
    #[case(b"10", 8)] // For octal numbers, missing zeros are prepended
    fn octal(#[case] input: &[u8], #[case] result: u8) {
        let (_, n) = parse_octal(input).unwrap();
        assert_eq!(n, result);
    }

    #[rstest]
    #[case(b"01", 1)]
    #[case(b"10", 16)]
    #[case(b"1", 16)] // For hex numbers, missing zeros are appended
    fn hex(#[case] input: &[u8], #[case] result: u8) {
        let (_, n) = parse_hexadecimal_bigram(input).unwrap();
        assert_eq!(n, result);
    }

    #[rstest]
    #[case(b"")]
    #[case(b"99")]
    #[should_panic]
    fn octal_failure(#[case] input: &[u8]) {
        parse_octal(input).unwrap();
    }

    #[rstest]
    #[case(b"")]
    #[case(b"g")]
    #[should_panic]
    fn hex_failure(#[case] input: &[u8]) {
        parse_hexadecimal_bigram(input).unwrap();
    }
}
