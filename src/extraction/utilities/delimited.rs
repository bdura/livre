use winnow::{
    combinator::{delimited, fail, terminated, trace},
    error::ContextError,
    token::{any, take},
    BStr, PResult, Parser,
};

use paste::paste;

use crate::extraction::{extract, Extract};

/// A parser that returns everything within a set of opening and closing bytes,
/// under the assumption that these are balanced.
///
/// What does "balanced" means? For instance, the following input is *not* balanced
/// (using parenthesis delimiters):
///
/// ```no-rust
/// (This is an incorrect PDF string, because of the lone `(` paranthesis.)
/// ```
///
/// The PDF specification allows nested delimiters, but there should be one closing
/// delimiter for each opening delimiter.
struct WithinBalancedParser {
    opening: u8,
    closing: u8,
}

impl WithinBalancedParser {
    fn new(opening: u8, closing: u8) -> Self {
        Self { opening, closing }
    }
}

impl<'a> Parser<&'a BStr, &'a [u8], ContextError> for WithinBalancedParser {
    fn parse_next(&mut self, input: &mut &'a BStr) -> PResult<&'a [u8], ContextError> {
        // Check that the first byte is an opening byte.
        // PERF: by relying on a `Parser` trait with implementation for common types
        // (here, `u8`), Winnow makes this quite easy
        self.opening.parse_next(input)?;

        // A `u8` counter would probably be amply sufficient already...
        // It seems unlikely to go beyond a few level of nesting, let alone 256.
        // Let us remain on the side of caution for now.
        // TODO: evaluate the need for a `u16` here.
        let mut counter: u16 = 1;

        for (i, &byte) in input.iter().enumerate() {
            if byte == self.closing {
                counter -= 1;
            } else if byte == self.opening {
                counter += 1;
            } else {
                continue;
            }

            if counter == 0 {
                // We need to consume the closing byte, without returning it - hence the
                // `terminated(_, any)` parser: we just checked that the next byte is a closing
                // token.
                return terminated(take(i), any).parse_next(input);
            }
        }

        // Delimiters are imbalances, we can just fail at this point.
        fail(input)
    }
}

/// Consume the inside of a balanced delimited input.
///
/// Adapted from <https://stackoverflow.com/questions/70630556/parse-allowing-nested-parentheses-in-nom>,
/// and recoded to match Winnow's approach.
fn take_within_balanced<'a>(
    opening: u8,
    closing: u8,
) -> impl Parser<&'a BStr, &'a [u8], ContextError> {
    WithinBalancedParser::new(opening, closing)
}

macro_rules! delimited {
    ($($name:ident: $opening:literal -> $closing:literal)+) => {
        $(
            paste!{
                #[derive(Debug, PartialEq)]
                pub struct $name<'de>(pub &'de BStr);

                impl<'de> Extract<'de> for $name<'de> {
                    fn extract(input: &mut &'de BStr) -> PResult<Self> {
                        trace(
                            stringify!(livre-[<$name:snake>]),
                            take_within_balanced($opening, $closing)
                                .map(|inside| Self(inside.as_ref())),
                        ).parse_next(input)
                    }
                }
            }
        )+
    };
}

delimited!(
    Brackets: b'[' -> b']'
    Parentheses: b'(' -> b')'
    Angles: b'<' -> b'>'
);

#[derive(Debug, PartialEq)]
pub struct DoubleAngles<'de>(pub &'de BStr);

impl<'de> Extract<'de> for DoubleAngles<'de> {
    fn extract(input: &mut &'de BStr) -> PResult<Self> {
        let Angles(inside) = delimited(b'<', extract, b'>').parse_next(input)?;
        Ok(Self(inside))
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(b'<', b'>', b"<test>", b"test")]
    #[case(b'<', b'>', b"<>", b"")]
    #[case(b'<', b'>', b"<te<s>t>", b"te<s>t")]
    #[case(b'<', b'>', b"<te<s>eafwt>", b"te<s>eafwt")]
    #[case(b'(', b')', b"(te<s>eafwt)", b"te<s>eafwt")]
    #[case(b'[', b']', b"[te<s>eafwt]", b"te<s>eafwt")]
    fn delimited(
        #[case] opening: u8,
        #[case] closing: u8,
        #[case] input: &[u8],
        #[case] expected: &[u8],
    ) {
        let mut parser = take_within_balanced(opening, closing);
        let input = &mut input.as_ref();
        let res = parser.parse_next(input).unwrap();

        assert_eq!(res, expected);
        assert!(input.is_empty());
    }

    #[rstest]
    #[case(b"<test>", Angles(b"test".as_ref().into()))]
    #[case(b"[test]", Brackets(b"test".as_ref().into()))]
    #[case(b"(test)", Parentheses(b"test".as_ref().into()))]
    #[case(b"<<test>>", DoubleAngles(b"test".as_ref().into()))]
    fn delimiters<'a, T>(#[case] input: &'a [u8], #[case] expected: T)
    where
        T: Extract<'a> + Debug + PartialEq,
    {
        let res = T::extract(&mut input.as_ref()).unwrap();
        assert_eq!(res, expected);
    }
}
