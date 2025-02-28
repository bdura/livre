use std::cell::RefCell;

use winnow::{
    ascii::multispace0,
    combinator::{alt, empty, fail, opt, preceded},
    dispatch,
    error::{ContextError, ErrMode},
    token::{any, take_till, take_until, take_while},
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
    pending: Option<Operator>,
}

// enum ObjectOrOperator {
//     Object(Object),
//     Operator(Vec<u8>),
// }
//
// impl Extract<'_> for ObjectOrOperator {
//     fn extract(input: &mut &'_ BStr) -> PResult<Self> {
//         trace(
//             "livre-content-atom",
//             alt((
//                 Object::extract.map(ObjectOrOperator::Object),
//                 take_till(1..=2, b" \t\r\n").map(|s: &BStr| ObjectOrOperator::Operator(s.to_vec())),
//             )),
//         )
//         .parse_next(input)
//     }
// }

fn build_operator<T>(arguments: &mut Vec<Object>) -> Result<Operator, OperatorError>
where
    T: FromArgs + Into<Operator>,
{
    let inner = T::from_args(arguments)?;
    Ok(inner.into())
}

impl Parser<&BStr, Operator, ContextError> for OperatorsAccumulator {
    fn parse_next(&mut self, input: &mut &BStr) -> PResult<Operator, ContextError> {
        if let Some(pending) = self.pending.take() {
            return Ok(pending);
        }

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
                return Ok(Operator::NotImplemented);
            }
        };

        if !self.arguments.is_empty() {
            self.arguments.clear();
            self.pending = Some(operator);
            Ok(Operator::NotImplemented)
        } else {
            Ok(operator)
        }
    }
}

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

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use rstest::rstest;
    use winnow::combinator::repeat;

    use crate::content::operators::text::RenderingMode;

    use super::*;

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
    #[case(b"2 Tr", SetTextRenderingMode(RenderingMode::FillThenStroke))]
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
    #[case(b"BT 1.0 1.0 Tc ET", vec![op!(BT).into(), Operator::NotImplemented, op!(1.0 Tc).into(), op!(ET).into()])]
    fn iterable(#[case] input: &[u8], #[case] expected: Vec<Operator>) {
        let mut parser = repeat(1.., OperatorsAccumulator::default());
        let res: Vec<Operator> = parser.parse_next(&mut input.as_ref()).unwrap();
        assert_eq!(res, expected);
    }
}
