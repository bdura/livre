mod text;

use text::{
    BeginText, EndText, MoveByOffset, MoveByOffsetAndSetLeading, MoveToNextLine,
    MoveToNextLineAndShowText, MoveToNextLineAndShowTextWithSpacing, SetCharacterSpacing,
    SetFontAndFontSize, SetHorizontalScaling, SetTextLeading, SetTextMatrix, SetTextRenderingMode,
    SetTextRise, SetWordSpacing, ShowText, ShowTextArray,
};
use winnow::{
    ascii::multispace0,
    combinator::{opt, trace},
    error::{ContextError, ErrMode},
    token::take_till,
    BStr, PResult, Parser,
};

use crate::extraction::{extract, Extract, Object};

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
    NotImplemented,
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

trait FromArgs<'a>: Extract<'a> + Into<Operator> {
    /// The number of arguments needed to build the operator.
    const N_ARGS: usize;

    /// Get the relevant context to build the operator.
    ///
    /// In the PDF specification, the operands are declared **before** the operator.
    /// That means that we need to track the operands without knowledge of their type.
    ///
    /// To that end, we just skip over the operands until we reach the operator,
    /// keeping track of their position in a `Vec<&'a BStr>`.
    ///
    /// A previous implementation used a `Vec<Object>` to store the operands, but that appoach
    /// had signigicant drawbacks:
    ///
    /// - we needed to implement `From<Object>` for every type that could be an operand.
    /// - building an [`Object`] is not cheap, since we need to try multiple suptypes.
    ///   In comparision, skipping over an operand is much faster.
    /// - we could not reuse the [`Extract`] trait.
    fn get_arguments(arguments: &mut Vec<&'a BStr>) -> PResult<&'a BStr> {
        arguments
            .iter()
            .rev()
            .nth(Self::N_ARGS - 1)
            .copied()
            .ok_or(ErrMode::Cut(ContextError::new()))
    }
    /// Extract the operator from the arguments. Returns an error if not enough arguments were
    /// found.
    fn from_args(arguments: &mut Vec<&'a BStr>) -> PResult<Self> {
        let input = &mut Self::get_arguments(arguments)?;
        extract(input)
    }
    fn extract_operator(arguments: &mut Vec<&'a BStr>) -> PResult<Operator> {
        Self::from_args(arguments).map(Into::into)
    }
}

#[macro_export]
macro_rules! impl_from_args {
    ($t:ty: $n:literal) => {
        impl FromArgs<'_> for $t {
            const N_ARGS: usize = $n;
        }
    };
}

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

fn parse_operator(input: &mut &BStr) -> PResult<Operator> {
    let mut arguments: Vec<&BStr> = Vec::with_capacity(4);

    multispace0(input)?;
    let mut cursor = *input;

    // If the input can be parsed as an object, then it is an argument.
    while opt(Object::recognize).parse_next(input)?.is_some() {
        arguments.push(cursor);

        multispace0(input)?;
        cursor = *input;
    }

    multispace0(input)?;
    let op = take_till(1..=2, b" \t\n\r").parse_next(input)?;

    let operator = match op {
        b"BT" => BeginText.into(),
        b"ET" => EndText.into(),
        // Text state operators
        b"Tc" => SetCharacterSpacing::extract_operator(&mut arguments)?,
        b"Tw" => SetWordSpacing::extract_operator(&mut arguments)?,
        b"Tz" => SetHorizontalScaling::extract_operator(&mut arguments)?,
        b"TL" => SetTextLeading::extract_operator(&mut arguments)?,
        b"Tf" => SetFontAndFontSize::extract_operator(&mut arguments)?,
        b"Tr" => SetTextRenderingMode::extract_operator(&mut arguments)?,
        b"Ts" => SetTextRise::extract_operator(&mut arguments)?,
        // Text positioning operators
        b"Td" => MoveByOffset::extract_operator(&mut arguments)?,
        b"TD" => MoveByOffsetAndSetLeading::extract_operator(&mut arguments)?,
        b"Tm" => SetTextMatrix::extract_operator(&mut arguments)?,
        b"T*" => MoveToNextLine::extract_operator(&mut arguments)?,
        b"Tj" => ShowText::extract_operator(&mut arguments)?,
        b"'" => MoveToNextLineAndShowText::extract_operator(&mut arguments)?,
        b"\"" => MoveToNextLineAndShowTextWithSpacing::extract_operator(&mut arguments)?,
        b"TJ" => ShowTextArray::extract_operator(&mut arguments)?,
        _ => {
            arguments.clear();
            Operator::NotImplemented
        }
    };

    Ok(operator)
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use rstest::rstest;

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
