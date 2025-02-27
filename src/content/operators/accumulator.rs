use winnow::{
    ascii::multispace0,
    combinator::{empty, fail, opt, preceded},
    dispatch,
    error::{ContextError, ErrMode},
    token::{any, take_till, take_until, take_while},
    BStr, PResult, Parser,
};

use crate::{
    content::operators::text::{BeginText, SetWordSpacing},
    extraction::{Extract, Object},
};

use super::Operator;

///
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

impl Parser<&BStr, Operator, ContextError> for OperatorsAccumulator {
    fn parse_next(&mut self, input: &mut &BStr) -> PResult<Operator, ContextError> {
        if let Some(pending) = self.pending.take() {
            return Ok(pending);
        }

        // If the input can be parsed as an object, then it is an argument.
        while let Some(argument) = opt(preceded(multispace0, Object::extract)).parse_next(input)? {
            self.arguments.push(argument)
        }

        let op: Operator = dispatch! {preceded(multispace0, any);
        // We need to introduce a `FromArguments` trait.
            b'B' => b'T'.value(BeginText).map(Operator::from),
            _ => fail,
        }
        .parse_next(input)?;

        if !self.arguments.is_empty() {
            self.pending = Some(op);
            Ok(Operator::NotImplemented)
        } else {
            Ok(op)
        }
    }
}
