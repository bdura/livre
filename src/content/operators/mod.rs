mod text;

use text::{
    BeginText, EndText, MoveByOffset, MoveByOffsetAndSetLeading, MoveToNextLine,
    MoveToNextLineAndShowText, MoveToNextLineAndShowTextWithSpacing, SetCharacterSpacing,
    SetFontAndFontSize, SetHorizontalScaling, SetTextLeading, SetTextMatrix, SetTextRenderingMode,
    SetTextRise, SetWordSpacing, ShowText, ShowTextArray,
};

use winnow::{
    ascii::multispace0,
    combinator::{fail, peek, preceded, repeat, trace},
    dispatch,
    token::any,
    BStr, PResult, Parser,
};

use crate::extraction::{take_till_delimiter, Angles, Brackets, Extract, Name, Parentheses};

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Operator {
    SetCharacterSpacing(SetCharacterSpacing),
    SetWordSpacing(SetWordSpacing),
    SetHorizontalScaling(SetHorizontalScaling),
    SetTextLeading(SetTextLeading),
    SetFontAndFontSize(SetFontAndFontSize),
    SetTextRenderingMode(SetTextRenderingMode),
    SetTextRise(SetTextRise),
    BeginText(BeginText),
    EndText(EndText),
    MoveByOffset(MoveByOffset),
    MoveByOffsetAndSetLeading(MoveByOffsetAndSetLeading),
    SetTextMatrix(SetTextMatrix),
    MoveToNextLine(MoveToNextLine),
    ShowText(ShowText),
    MoveToNextLineAndShowText(MoveToNextLineAndShowText),
    MoveToNextLineAndShowTextWithSpacing(MoveToNextLineAndShowTextWithSpacing),
    ShowTextArray(ShowTextArray),
    NotImplemented(String),
}

macro_rules! impl_from {
    ($($t:ident,)+) => {
        $(
            impl From<$t> for Operator {
                fn from(value: $t) -> Self {
                    Self::$t(value)
                }
            }
        )+
    };
}

impl_from!(
    SetCharacterSpacing,
    SetWordSpacing,
    SetHorizontalScaling,
    SetTextLeading,
    SetFontAndFontSize,
    SetTextRenderingMode,
    SetTextRise,
    BeginText,
    EndText,
    MoveByOffset,
    MoveByOffsetAndSetLeading,
    SetTextMatrix,
    MoveToNextLine,
    ShowText,
    MoveToNextLineAndShowText,
    MoveToNextLineAndShowTextWithSpacing,
    ShowTextArray,
);

#[macro_export]
macro_rules! extract_tuple {
    ($name:ident: 0) => {
        impl<'de> Extract<'de> for $name {
            fn extract(_input: &mut &'_ winnow::BStr) -> winnow::PResult<Self> {
                Ok(Self)
            }
        }
    };
    ($name:ident: 1) => {
        impl<'de> Extract<'de> for $name {
            fn extract(input: &mut &'_ winnow::BStr) -> winnow::PResult<Self> {
                extract.map(Self).parse_next(input)
            }
        }
    };
    ($name:ident: 2) => {
        impl<'de> Extract<'de> for $name {
            fn extract(input: &mut &'_ winnow::BStr) -> winnow::PResult<Self> {
                let (a, b) = extract(input)?;
                Ok(Self(a, b))
            }
        }
    };
    ($name:ident: 3) => {
        impl<'de> Extract<'de> for $name {
            fn extract(input: &mut &'_ winnow::BStr) -> winnow::PResult<Self> {
                let (a, b, c) = extract(input)?;
                Ok(Self(a, b, c))
            }
        }
    };
    ($name:ident: 6) => {
        impl<'de> Extract<'de> for $name {
            fn extract(input: &mut &'_ winnow::BStr) -> winnow::PResult<Self> {
                let (a, b, c, d, e, f) = extract(input)?;
                Ok(Self(a, b, c, d, e, f))
            }
        }
    };
}

impl Extract<'_> for Operator {
    fn extract(input: &mut &BStr) -> PResult<Self> {
        trace("livre-operator", parse_operator).parse_next(input)
    }
}

/// Recognize an operand, without parsing it.
fn recognize_operand<'de>(input: &mut &'de BStr) -> PResult<&'de [u8]> {
    dispatch! {peek(any);
        b'/' => Name::recognize,
        b'[' => Brackets::recognize,
        b'(' => Parentheses::recognize,
        b'<' => Angles::recognize,
        b'+' | b'-' | b'.' | b'0'..=b'9' => take_till_delimiter(1..),
        _ => fail
    }
    .parse_next(input)
}

/// Helper function that extracts an operator and converts it to [`Operator`].
fn extract_operator<'a, T>(input: &mut &'a BStr) -> PResult<Operator>
where
    T: Extract<'a>,
    Operator: From<T>,
{
    T::extract.map(Operator::from).parse_next(input)
}

fn parse_operator(input: &mut &BStr) -> PResult<Operator> {
    let mut cursor = *input;

    repeat(0.., preceded(multispace0, recognize_operand))
        .map(|()| ())
        .parse_next(input)?;

    let op = preceded(multispace0, take_till_delimiter(1..=3)).parse_next(input)?;

    let operator = match op {
        // Text object operators
        b"BT" => BeginText.into(),
        b"ET" => EndText.into(),
        // Text state operators
        b"Tc" => extract_operator::<SetCharacterSpacing>(&mut cursor)?,
        b"Tw" => extract_operator::<SetWordSpacing>(&mut cursor)?,
        b"Tz" => extract_operator::<SetHorizontalScaling>(&mut cursor)?,
        b"TL" => extract_operator::<SetTextLeading>(&mut cursor)?,
        b"Tf" => extract_operator::<SetFontAndFontSize>(&mut cursor)?,
        b"Tr" => extract_operator::<SetTextRenderingMode>(&mut cursor)?,
        b"Ts" => extract_operator::<SetTextRise>(&mut cursor)?,
        // Text positioning operators
        b"Td" => extract_operator::<MoveByOffset>(&mut cursor)?,
        b"TD" => extract_operator::<MoveByOffsetAndSetLeading>(&mut cursor)?,
        b"Tm" => extract_operator::<SetTextMatrix>(&mut cursor)?,
        b"T*" => extract_operator::<MoveToNextLine>(&mut cursor)?,
        b"Tj" => extract_operator::<ShowText>(&mut cursor)?,
        b"'" => extract_operator::<MoveToNextLineAndShowText>(&mut cursor)?,
        b"\"" => extract_operator::<MoveToNextLineAndShowTextWithSpacing>(&mut cursor)?,
        b"TJ" => extract_operator::<ShowTextArray>(&mut cursor)?,
        _ => Operator::NotImplemented(String::from_utf8_lossy(op).to_string()),
    };

    Ok(operator)
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use rstest::rstest;

    use crate::extraction::extract;

    use super::*;

    macro_rules! op {
        (BT) => {
            BeginText
        };
        (ET) => {
            EndText
        };
        ($x:literal Tc) => {
            SetCharacterSpacing($x)
        };
        ($x:literal Tw) => {
            SetWordSpacing($x)
        };
        ($x:literal Tz) => {
            SetHorizontalScaling($x)
        };
        ($x:literal TL) => {
            SetTextLeading($x)
        };
        ($x:literal $y:literal Tf) => {
            SetFontAndFontSize($x, $y)
        };
        ($x:literal Tf) => {
            SetFontAndFontSize($x)
        };
        ($x:literal Tr) => {
            SetTextRenderingMode($x)
        };
        ($x:literal Ts) => {
            SetTextRise($x)
        };
        ($x:literal $y:literal Td) => {
            MoveByOffset($x, $y)
        };
        ($x:literal $y:literal TD) => {
            MoveByOffsetAndSetLeading($x, $y)
        };
        ($a:literal $b:literal $c:literal $d:literal $e:literal $f:literal Tm) => {
            SetTextMatrix($a, $b, $c, $d, $e, $f)
        };
        (T*) => {
            MoveToNextLine
        };
        ($x:literal Tj) => {
            ShowText($x)
        };
    }

    #[rstest]
    #[case(op!(BT), BeginText)]
    #[case(op!(ET), EndText)]
    #[case(op!(0.12 Tc), SetCharacterSpacing(0.12))]
    #[case(op!(1.12 Tw), SetWordSpacing(1.12))]
    #[case(op!(T*), MoveToNextLine)]
    #[case(op!(1.0 2.0 TD), MoveByOffsetAndSetLeading(1.0, 2.0))]
    fn test_macro<O>(#[case] input: O, #[case] expected: O)
    where
        O: PartialEq + Debug,
    {
        assert_eq!(input, expected);
    }

    #[rstest]
    #[case(b"BT", op!(BT))]
    #[case(b"ET", op!(ET))]
    #[case(b"0.12 Tc", op!(0.12 Tc))]
    #[case(b"1.0 Tw", op!(1.0 Tw))]
    #[case(b"T*", MoveToNextLine)]
    fn units<O>(#[case] input: &[u8], #[case] expected: O)
    where
        O: Into<Operator>,
    {
        let result = extract(&mut input.as_ref()).unwrap();
        let expected: Operator = expected.into();
        assert_eq!(expected, result);
    }
}
