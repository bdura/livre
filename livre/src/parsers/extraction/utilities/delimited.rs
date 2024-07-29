use crate::parsers::take_within_balanced;

use super::super::{extract, Extract};

macro_rules! delimited {
    ($Ty:tt, $left:literal, $right:literal) => {
        #[derive(Debug, PartialEq, Eq, Clone)]
        pub struct $Ty<'input>(pub &'input [u8]);

        impl<'input> Extract<'input> for $Ty<'input> {
            fn extract(input: &'input [u8]) -> nom::IResult<&'input [u8], Self> {
                let (input, value) = take_within_balanced($left, $right)(input)?;
                Ok((input, Self(value)))
            }
        }
    };
}

delimited!(Parentheses, b'(', b')');
delimited!(Brackets, b'[', b']');
delimited!(Angles, b'<', b'>');

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DoubleAngles<'input>(pub &'input [u8]);

impl<'input> Extract<'input> for DoubleAngles<'input> {
    fn extract(input: &'input [u8]) -> nom::IResult<&'input [u8], Self> {
        let (input, Angles(value)) = extract(input)?;
        let (r, Angles(value)) = extract(value)?;

        assert!(r.is_empty());

        Ok((input, Self(value)))
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(b"()", b"")]
    #[case(b"(test)", b"test")]
    #[case(b"(te(st))", b"te(st)")]
    fn parentheses(#[case] input: &[u8], #[case] expected: &[u8]) {
        let (_, Parentheses(value)) = extract(input).unwrap();
        assert_eq!(value, expected);
    }

    #[rstest]
    #[case(b"[]", b"")]
    #[case(b"[test]", b"test")]
    #[case(b"[()]", b"()")]
    fn brackets(#[case] input: &[u8], #[case] expected: &[u8]) {
        let (_, Brackets(value)) = extract(input).unwrap();
        assert_eq!(value, expected);
    }

    #[rstest]
    #[case(b"<>", b"")]
    #[case(b"<()>", b"()")]
    #[case(b"<test>", b"test")]
    fn angles(#[case] input: &[u8], #[case] expected: &[u8]) {
        let (_, Angles(value)) = extract(input).unwrap();
        assert_eq!(value, expected);
    }

    #[rstest]
    #[case(b"<<>>", b"")]
    #[case(b"<<<<>>>>", b"<<>>")]
    #[case(b"<<test>>", b"test")]
    fn double_angles(#[case] input: &[u8], #[case] expected: &[u8]) {
        let (_, DoubleAngles(value)) = extract(input).unwrap();
        assert_eq!(value, expected);
    }
}
