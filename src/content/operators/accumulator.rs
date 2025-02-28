use winnow::{
    ascii::multispace0,
    combinator::{opt, preceded},
    error::ContextError,
    token::take_till,
    BStr, PResult, Parser,
};

use crate::{
    content::operators::text::{BeginText, EndText, SetCharacterSpacing, SetWordSpacing},
    extraction::{Extract, Object},
};

use super::{
    text::{
        SetFontAndFontSize, SetHorizontalScaling, SetTextLeading, SetTextRenderingMode, SetTextRise,
    },
    Operator,
};
use super::{FromArgs, OperatorError};

#[derive(Debug, Default)]
pub struct OperatorsAccumulator {
    /// Accumulate arguments in `self.arguments`
    arguments: Vec<Object>,
}

fn build_operator<T>(arguments: &mut Vec<Object>) -> Result<Operator, OperatorError>
where
    T: FromArgs + Into<Operator>,
{
    let inner = T::from_args(arguments)?;
    Ok(inner.into())
}

impl Parser<&BStr, Operator, ContextError> for OperatorsAccumulator {
    fn parse_next(&mut self, input: &mut &BStr) -> PResult<Operator, ContextError> {
        // If the input can be parsed as an object, then it is an argument.
        while let Some(argument) = opt(preceded(multispace0, Object::extract)).parse_next(input)? {
            self.arguments.push(argument)
        }

        multispace0(input)?;
        let op = take_till(1..=2, b" \t\n\r").parse_next(input)?;

        let operator = match op {
            b"BT" => BeginText.into(),
            b"ET" => EndText.into(),
            b"Tc" => build_operator::<SetCharacterSpacing>(&mut self.arguments)?,
            b"Tw" => build_operator::<SetWordSpacing>(&mut self.arguments)?,
            b"Tz" => build_operator::<SetHorizontalScaling>(&mut self.arguments)?,
            b"TL" => build_operator::<SetTextLeading>(&mut self.arguments)?,
            b"Tf" => build_operator::<SetFontAndFontSize>(&mut self.arguments)?,
            b"Tr" => build_operator::<SetTextRenderingMode>(&mut self.arguments)?,
            b"Ts" => build_operator::<SetTextRise>(&mut self.arguments)?,
            _ => {
                self.arguments.clear();
                Operator::NotImplemented
            }
        };

        Ok(operator)
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use rstest::rstest;
    use winnow::combinator::repeat;

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
    }

    #[rstest]
    #[case(op!(BT), BeginText)]
    #[case(op!(ET), EndText)]
    #[case(op!(0.12 Tc), SetCharacterSpacing(0.12))]
    #[case(op!(1.12 Tw), SetWordSpacing(1.12))]
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
    fn units<O>(#[case] input: &[u8], #[case] expected: O)
    where
        O: Into<Operator>,
    {
        let mut parser = repeat(1.., OperatorsAccumulator::default());
        let res: Vec<Operator> = parser.parse_next(&mut input.as_ref()).unwrap();

        let expected: Operator = expected.into();
        assert_eq!(res[0], expected);
    }

    #[rstest]
    #[case(b"BT 1.0 Tc ET", vec![op!(BT).into(), op!(1.0 Tc).into(), op!(ET).into()])]
    #[case(b"BT Xa 1.0 Tc ET", vec![op!(BT).into(), Operator::NotImplemented, op!(1.0 Tc).into(), op!(ET).into()])]
    fn iterable(#[case] input: &[u8], #[case] expected: Vec<Operator>) {
        let mut parser = repeat(1.., OperatorsAccumulator::default());
        let res: Vec<Operator> = parser.parse_next(&mut input.as_ref()).unwrap();
        assert_eq!(res, expected);
    }
}
