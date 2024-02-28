use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{digit0, digit1},
    combinator::{opt, recognize},
    sequence::{separated_pair, tuple},
    IResult,
};

use crate::utilities::parse_sign;

use super::Extract;

fn recognize_real(input: &[u8]) -> IResult<&[u8], (&[u8], &[u8])> {
    alt((
        separated_pair(digit1, tag("."), digit0),
        separated_pair(digit0, tag("."), digit1),
    ))(input)
}

// TODO: handle parsing error.
macro_rules! extract {
    ($type:ty) => {
        impl Extract for $type {
            fn extract(input: &[u8]) -> IResult<&[u8], Self> {
                let (input, num) = recognize(tuple((opt(parse_sign), recognize_real)))(input)?;

                // SAFETY: num is an optional sign, followed by digits with a single point.
                let num = unsafe { std::str::from_utf8_unchecked(num) };

                let n = num.parse().unwrap();

                Ok((input, n))
            }
        }
    };
}

extract!(f32);
extract!(f64);

#[cfg(test)]
mod tests {
    use crate::extraction::Parse;
    use rstest::rstest;

    #[rstest]
    #[case(b"1.", 1.0)]
    #[case(b"+1.", 1.0)]
    #[case(b"-1.", -1.0)]
    #[case(b"-.1", -0.1)]
    #[case(b".0", 0.0)]
    #[case(b"+0.", 0.0)]
    #[case(b"12345.6789", 12345.6789)]
    #[case(b"-12345.6789", -12345.6789)]
    fn parse_unsigned(#[case] input: &[u8], #[case] result: f64) {
        assert_eq!(result, input.parse().unwrap());

        let r = result as f32;
        assert_eq!(r, input.parse().unwrap());
    }
}
