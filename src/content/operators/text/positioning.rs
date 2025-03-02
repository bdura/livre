use winnow::{BStr, PResult};

use crate::{
    content::operators::FromArgs,
    extract_tuple,
    extraction::{extract, Extract},
    impl_from_args,
};

/// `Td` operator.
///
/// Move to the start of the next line, offset from the start of the current line
/// by `(tx, ty)`. `tx` and `ty` shall denote numbers expressed in unscaled
/// text space units
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveByOffset(f32, f32);

extract_tuple!(MoveByOffset: 2);
impl_from_args!(MoveByOffset: 2);

/// `TD` operator.
///
/// Move to the start of the next line, offset from the start of the current line
/// by `(tx, ty)`. As a side effect, this operator shall set the leading parameter
/// in the text state. Equivalent to:
///
/// ```raw
/// -ty TL
/// tx ty Td
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveByOffsetAndSetLeading(pub(crate) f32, pub(crate) f32);

extract_tuple!(MoveByOffsetAndSetLeading: 2);
impl_from_args!(MoveByOffsetAndSetLeading: 2);

/// `Tm` operator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SetTextMatrix(f32, f32, f32, f32, f32, f32);

extract_tuple!(SetTextMatrix: 6);
impl_from_args!(SetTextMatrix: 6);

/// `T*` operator
///
/// Move to the start of the next line. Equivalent to:
///
/// ```raw
/// 0 -T_l Td
/// ```
///
/// where `T_l` denotes the current leading parameter in the text state.
/// The negative of T l is used here because T l is the text leading expressed
/// as a positive number. Going to the next line entails decreasing the y coordinate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveToNextLine;

extract_tuple!(MoveToNextLine: 0);

impl FromArgs<'_> for MoveToNextLine {
    const N_ARGS: usize = 0;
    fn extract_operator(_: &mut Vec<&'_ BStr>) -> PResult<crate::content::Operator> {
        Ok(Self.into())
    }
}
