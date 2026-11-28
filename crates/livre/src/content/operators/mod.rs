//! Operators used in content streams.
//!
//! This module defines the operators that are used in content streams, as well as specific
//! behaviour. The operators are organised in a hierarchy that follows the PDF specification.
//!
//! Note that PDF documents serialise operators as zero or more operands followed by an operator
//! identifier. This means that we *need* to parse the operands before we can determine the
//! operator.
//!
//! To achieve this, we could use two different approaches:
//!
//! 1. Parse each operand as a PDF [`Object`](crate::extraction::Object), and then parse the operator.
//! 2. Merely *recognize* the operands, parse the operator, and finally go back and parse the operands.
//!
//! Livre uses the second approach, since it is more efficient. Indeed, parsing an `Object`
//! requires going through every alternative until one matches, while skipping over the operands
//! allows us to use the right parser directly, which is possible once we know the operator.

pub mod graphics;
pub mod text;

use graphics::{ConcatenateMatrix, GraphicsStateOperator, RestoreGraphicsState, SaveGraphicsState};

use text::{
    BeginText, EndText, MoveByOffset, MoveByOffsetAndSetLeading, MoveToNextLine,
    MoveToNextLineAndShowText, MoveToNextLineAndShowTextWithSpacing, SetCharacterSpacing,
    SetFontAndFontSize, SetHorizontalScaling, SetTextLeading, SetTextMatrix, SetTextRenderingMode,
    SetTextRise, SetWordSpacing, ShowText, ShowTextArray, TextOperator,
};

use winnow::{
    combinator::{fail, peek, preceded, repeat, trace},
    dispatch,
    error::ErrMode,
    token::any,
    BStr, ModalResult, Parser,
};

use crate::extraction::{
    multicomment0, recognize_angled, take_till_delimiter, Brackets, Extract, Name, Parentheses,
};

/// Content stream operator.
///
/// ## Implementation notes
///
/// Although Livre defines a somewhat deep hierarchy of operators, the extraction is exlusively
/// driven by `Operator`. This allows a more efficient extraction of operators, since we do not
/// have to rely on nested alternatives or other combinators to parse the operator.
///
/// Moreover:
///
/// - intermerdiate operators in the hierarchy do not implement [`Extract`], for the reason
///   established above.
/// - leaf operators do implement [`Extract`], although partially - they only know how to parse
///   their operands.
///
/// Finally, the `Operator` enum is not exhaustive, since it does not (yet!) cover all operators
/// defined by the PDF specification: text operators and the special graphics state operators
/// (`q`, `Q`, `cm`) are modelled, everything else is [`NotImplemented`](Operator::NotImplemented).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Operator {
    // Delimiters
    BeginText(BeginText),
    EndText(EndText),
    Text(TextOperator),
    GraphicsState(GraphicsStateOperator),
    /// An operator livre does not model, with its operands dropped and its name kept.
    NotImplemented(OperatorName),
}

/// The name of an operator livre does not model, such as `re`, `f` or `BDC`.
///
/// An operator token is one to three bytes, held inline: building one never allocates.
/// [`Display`](std::fmt::Display) writes the bytes as UTF-8, replacing invalid sequences
/// with U+FFFD.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperatorName {
    bytes: [u8; 3],
    len: u8,
}

impl OperatorName {
    /// Keeps the first three bytes of `name`, the longest an operator token can be.
    fn new(name: &[u8]) -> Self {
        let len = name.len().min(3);
        let mut bytes = [0; 3];
        bytes[..len].copy_from_slice(&name[..len]);
        Self {
            bytes,
            len: len as u8,
        }
    }

    /// The operator's bytes, as read from the content stream.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..usize::from(self.len)]
    }
}

impl std::fmt::Display for OperatorName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&String::from_utf8_lossy(self.as_bytes()))
    }
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

impl_from!(BeginText, EndText,);

impl Extract<'_> for Operator {
    fn extract(input: &mut &BStr) -> ModalResult<Self> {
        trace("livre-operator", parse_operator).parse_next(input)
    }
}

/// Recognize an operand, without parsing it. Thanks to the
/// [`recognize`](Extract::recognize) static method, we can efficiently skip over the operands.
fn recognize_operand<'de>(input: &mut &'de BStr) -> ModalResult<&'de [u8]> {
    dispatch! {peek(any);
        b'/' => Name::recognize,
        b'[' => Brackets::recognize,
        b'(' => Parentheses::recognize,
        b'<' => recognize_angled,
        b'+' | b'-' | b'.' | b'0'..=b'9' => take_till_delimiter(1..).map_err(ErrMode::Backtrack),
        _ => fail
    }
    .parse_next(input)
}

/// Helper function that extracts an operator and converts it to [`Operator`].
fn extract_operator<'a, T>(input: &mut &'a BStr) -> ModalResult<Operator>
where
    T: Extract<'a>,
    Operator: From<T>,
{
    T::extract.map(Operator::from).parse_next(input)
}

/// Parse an operator from the input.
///
/// Defining the parsing logic at the [`Operator`] level allows us to use a single alternative,
/// which is more efficient than using nested parsing logic.
fn parse_operator(input: &mut &BStr) -> ModalResult<Operator> {
    let mut cursor = *input;

    repeat(0.., preceded(multicomment0, recognize_operand))
        .map(|()| ())
        .parse_next(input)?;

    let op = preceded(
        multicomment0,
        take_till_delimiter(1..=3).map_err(ErrMode::Backtrack),
    )
    .parse_next(input)?;

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
        // Special graphics state operators
        b"q" => SaveGraphicsState.into(),
        b"Q" => RestoreGraphicsState.into(),
        b"cm" => extract_operator::<ConcatenateMatrix>(&mut cursor)?,
        _ => Operator::NotImplemented(OperatorName::new(op)),
    };

    Ok(operator)
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use rstest::rstest;

    use crate::{content::state::Matrix, extraction::extract};

    use super::*;

    macro_rules! op {
        ($text:literal) => {
            extract(&mut $text.as_ref()).unwrap()
        };
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
    #[case(b"q", SaveGraphicsState)]
    #[case(b"Q", RestoreGraphicsState)]
    #[case(b"1 0 0 -1 0 792 cm", ConcatenateMatrix(Matrix::new(1.0, 0.0, 0.0, -1.0, 0.0, 792.0)))]
    #[case(
        b".75 0 0 .75 72 86.546265 cm",
        ConcatenateMatrix(Matrix::new(0.75, 0.0, 0.0, 0.75, 72.0, 86.546265))
    )]
    fn graphics_state_operators<O>(#[case] input: &[u8], #[case] expected: O)
    where
        O: Into<Operator>,
    {
        let result = extract(&mut input.as_ref()).unwrap();
        let expected: Operator = expected.into();
        assert_eq!(expected, result);
    }

    #[rstest]
    #[case(b"BT", op!(BT))]
    #[case(b"ET", op!(ET))]
    #[case(b"0.12 Tc", op!(0.12 Tc))]
    #[case(b"1.0 Tw", op!(1.0 Tw))]
    #[case(b"T*", MoveToNextLine)]
    #[case(b"1.0 % spacing\nTw", op!(1.0 Tw))]
    fn units<O>(#[case] input: &[u8], #[case] expected: O)
    where
        O: Into<Operator>,
    {
        let result = extract(&mut input.as_ref()).unwrap();
        let expected: Operator = expected.into();
        assert_eq!(expected, result);
    }

    #[test]
    fn comments_between_operators() {
        let mut input = BStr::new(b"% header\nBT % open\n1 Tc\n72 % x\n700 Td ET % close\n");
        let mut it =
            winnow::combinator::iterator(&mut input, preceded(multicomment0, Operator::extract));
        let operators: Vec<Operator> = (&mut it).collect();
        it.finish().unwrap();
        multicomment0(&mut input).unwrap();

        assert_eq!(
            operators,
            vec![
                op!(BT).into(),
                op!(1.0 Tc).into(),
                Operator::extract(&mut BStr::new(b"72 700 Td")).unwrap(),
                op!(ET).into(),
            ]
        );
        assert!(input.is_empty());
    }

    #[rstest]
    #[case(b"0 0 10 10 re", "re")]
    #[case(b"f", "f")]
    #[case(b"/Span <</MCID 0>> BDC", "BDC")]
    #[case(b"/Span <</ActualText (a>b)>> BDC", "BDC")]
    #[case(b"/Span <</ActualText <FEFF005D> /E (])>> BDC", "BDC")]
    #[case(b"W*", "W*")]
    fn unmodelled_operator_keeps_its_name(#[case] input: &[u8], #[case] name: &str) {
        let result: Operator = extract(&mut input.as_ref()).unwrap();
        let Operator::NotImplemented(op) = result else {
            panic!("expected NotImplemented, got {result:?}");
        };
        assert_eq!(op.as_bytes(), name.as_bytes());
        assert_eq!(op.to_string(), name);
    }

    #[test]
    fn operator_name_is_three_bytes_at_most() {
        let name = OperatorName::new(b"BDCX");
        assert_eq!(name.as_bytes(), b"BDC");
    }
}
