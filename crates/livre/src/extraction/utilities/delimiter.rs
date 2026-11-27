use winnow::{
    combinator::trace,
    error::{ContextError, ErrMode},
    stream::Range,
    token::{take, take_till},
    BStr, ModalResult, Parser,
};

use crate::extraction::Extract;

/// How deep arrays and dictionaries may nest inside one another before the scan gives up.
/// One bit of [`Open`] per level.
const MAX_DEPTH: u32 = u64::BITS;

/// A container whose end the scan looks for.
#[derive(Clone, Copy, PartialEq)]
enum Container {
    Array,
    Dictionary,
}

/// The containers opened and not yet closed, innermost last.
///
/// A bit per level rather than a `Vec`: the scan runs on every operand of a content
/// stream and must not allocate.
#[derive(Default)]
struct Open {
    /// Bit 0 is the innermost container: set for a dictionary, clear for an array.
    kinds: u64,
    depth: u32,
}

impl Open {
    /// `None` past [`MAX_DEPTH`] levels.
    fn push(&mut self, container: Container) -> Option<()> {
        if self.depth == MAX_DEPTH {
            return None;
        }
        self.kinds = (self.kinds << 1) | u64::from(container == Container::Dictionary);
        self.depth += 1;
        Some(())
    }

    /// `None` unless the innermost open container is a `container`.
    fn pop(&mut self, container: Container) -> Option<()> {
        if self.depth == 0 {
            return None;
        }
        let innermost = if self.kinds & 1 == 1 {
            Container::Dictionary
        } else {
            Container::Array
        };
        if innermost != container {
            return None;
        }
        self.kinds >>= 1;
        self.depth -= 1;
        Some(())
    }

    fn is_empty(&self) -> bool {
        self.depth == 0
    }
}

/// The length of the literal string `bytes` starts with, both parentheses included.
///
/// Balanced parentheses nest; a backslash escapes the byte after it, so `\(` and `\)`
/// do not count (ISO 32000-2, 7.3.4.2).
fn literal_string_len(bytes: &[u8]) -> Option<usize> {
    if bytes.first() != Some(&b'(') {
        return None;
    }

    let mut depth = 0usize;
    let mut bytes = bytes.iter().enumerate();
    while let Some((i, byte)) = bytes.next() {
        match byte {
            b'\\' => {
                bytes.next();
            }
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
    }

    None
}

/// The length of the hexadecimal string `bytes` starts with, both angle brackets included.
/// Nothing nests in a hexadecimal string: it ends at the first `>`.
fn hexadecimal_string_len(bytes: &[u8]) -> Option<usize> {
    if bytes.first() != Some(&b'<') {
        return None;
    }
    bytes.iter().position(|&byte| byte == b'>').map(|i| i + 1)
}

/// The length of the comment `bytes` starts with, up to its end of line and without it.
/// `None` if the line never ends: the container holding the comment is then never closed.
fn comment_len(bytes: &[u8]) -> Option<usize> {
    bytes.iter().position(|byte| matches!(byte, b'\r' | b'\n'))
}

/// The length of the array or dictionary `bytes` starts with, both delimiters included.
///
/// The scan is lexical: it skips literal strings, hexadecimal strings and comments as
/// whole tokens, so a delimiter inside one of them neither opens nor closes anything.
/// pdfTeX shows a lone bracket as `[([)]TJ`.
///
/// `None` if the input ends before the container does, if a container is closed by the
/// delimiter of the other kind, or if the nesting goes past [`MAX_DEPTH`].
fn container_len(bytes: &[u8]) -> Option<usize> {
    let mut open = Open::default();
    let mut i = 0;

    loop {
        let closed = match (*bytes.get(i)?, bytes.get(i + 1)) {
            (b'[', _) => {
                open.push(Container::Array)?;
                i += 1;
                false
            }
            (b'<', Some(b'<')) => {
                open.push(Container::Dictionary)?;
                i += 2;
                false
            }
            (b'<', _) => {
                i += hexadecimal_string_len(&bytes[i..])?;
                false
            }
            (b'(', _) => {
                i += literal_string_len(&bytes[i..])?;
                false
            }
            (b'%', _) => {
                i += comment_len(&bytes[i..])?;
                false
            }
            (b']', _) => {
                open.pop(Container::Array)?;
                i += 1;
                true
            }
            (b'>', Some(b'>')) => {
                open.pop(Container::Dictionary)?;
                i += 2;
                true
            }
            // A lone `>` closes nothing: hexadecimal strings are skipped whole.
            (b'>', _) => return None,
            _ => {
                i += 1;
                false
            }
        };

        if closed && open.is_empty() {
            return Some(i);
        }
    }
}

/// Take what `len` measures at the start of the input, which must begin with `opening`.
fn take_measured<'de>(
    opening: &'static [u8],
    len: fn(&[u8]) -> Option<usize>,
) -> impl Parser<&'de BStr, &'de [u8], ErrMode<ContextError>> {
    move |input: &mut &'de BStr| {
        let len = input
            .starts_with(opening)
            .then(|| len(input))
            .flatten()
            .ok_or_else(|| ErrMode::Backtrack(ContextError::new()))?;

        take(len).parse_next(input)
    }
}

/// All PDF delimiters.
static DELIMITERS: &[u8] = b"()<>[]{}/% \t\r\n";

/// Useful for recognizing elements.
pub fn take_till_delimiter<'a>(
    occurrences: impl Into<Range>,
) -> impl Parser<&'a BStr, &'a [u8], ContextError> {
    trace("livre-till-delimiter", take_till(occurrences, DELIMITERS))
}

/// Recognize what a `<` starts: a dictionary if another `<` follows, a hexadecimal string
/// otherwise. Their insides follow different rules, so the two cannot share a scan.
pub fn recognize_angled<'de>(input: &mut &'de BStr) -> ModalResult<&'de [u8]> {
    if input.starts_with(b"<<") {
        DoubleAngles::recognize(input)
    } else {
        trace(
            "livre-hexadecimal-string-bounds",
            take_measured(b"<", hexadecimal_string_len),
        )
        .parse_next(input)
    }
}

/// Defines a type holding what lies between two delimiters, the delimiters left out.
macro_rules! delimited {
    ($(#[$doc:meta])* $name:ident, $label:literal: $opening:literal ... $closing:literal, $len:path) => {
        $(#[$doc])*
        #[derive(Debug, PartialEq)]
        pub struct $name<'de>(pub &'de BStr);

        impl<'de> Extract<'de> for $name<'de> {
            fn extract(input: &mut &'de BStr) -> ModalResult<Self> {
                trace(
                    $label,
                    take_measured($opening, $len).map(|whole: &'de [u8]| {
                        Self(BStr::new(&whole[$opening.len()..whole.len() - $closing.len()]))
                    }),
                )
                .parse_next(input)
            }

            fn recognize(input: &mut &'de BStr) -> ModalResult<&'de [u8]> {
                trace($label, take_measured($opening, $len)).parse_next(input)
            }
        }
    };
}

delimited!(
    /// The inside of an array, `[…]`.
    Brackets, "livre-brackets": b"[" ... b"]", container_len
);
delimited!(
    /// The inside of a dictionary, `<<…>>`.
    DoubleAngles, "livre-double-angles": b"<<" ... b">>", container_len
);
delimited!(
    /// The inside of a literal string, `(…)`, escapes left as written.
    Parentheses, "livre-parentheses": b"(" ... b")", literal_string_len
);

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use rstest::rstest;

    use crate::extraction::extract;

    use super::*;

    #[rstest]
    #[case(b"(test)", b"test")]
    #[case(b"(te(s)t)", b"te(s)t")]
    #[case(b"(te\\)st)", b"te\\)st")]
    #[case(b"(te\\(st)", b"te\\(st")]
    #[case(b"(te<s>]%t)", b"te<s>]%t")]
    fn parentheses(#[case] input: &[u8], #[case] expected: &[u8]) {
        let mut input = BStr::new(input);
        let Parentheses(inside) = extract(&mut input).unwrap();

        assert_eq!(inside, expected);
        assert!(input.is_empty());
    }

    #[rstest]
    #[case(b"[test]", b"test")]
    #[case(b"[]", b"")]
    #[case(b"[1 [2 3] 4]", b"1 [2 3] 4")]
    // A closing or an opening bracket inside a literal string is data.
    #[case(b"[(a]b)]", b"(a]b)")]
    #[case(b"[([)]", b"([)")]
    #[case(b"[([0)]", b"([0)")]
    #[case(b"[(a\\)]b)]", b"(a\\)]b)")]
    #[case(b"[(a(])b)]", b"(a(])b)")]
    #[case(b"[(a>>b)]", b"(a>>b)")]
    // `%` inside a literal string does not start a comment.
    #[case(b"[(a%b)(c)]", b"(a%b)(c)")]
    // A comment runs to the end of its line, whatever it holds.
    #[case(b"[1 % ] ( < >>\n2]", b"1 % ] ( < >>\n2")]
    #[case(b"[1 % ]\r2]", b"1 % ]\r2")]
    // Hexadecimal strings and dictionaries nest in an array.
    #[case(b"[<ab> <cd>]", b"<ab> <cd>")]
    #[case(b"[<</A [1] /B (])>>]", b"<</A [1] /B (])>>")]
    fn brackets(#[case] input: &[u8], #[case] expected: &[u8]) {
        let mut input = BStr::new(input);
        let Brackets(inside) = extract(&mut input).unwrap();

        assert_eq!(inside, expected);
        assert!(input.is_empty());
    }

    #[rstest]
    #[case(b"<<test>>", b"test")]
    #[case(b"<<>>", b"")]
    #[case(b"<</A <</B 1>>>>", b"/A <</B 1>>")]
    #[case(b"<</T (a>b)>>", b"/T (a>b)")]
    #[case(b"<</T (a>>b)>>", b"/T (a>>b)")]
    #[case(b"<</A 1 % x > y\n>>", b"/A 1 % x > y\n")]
    #[case(b"<</A 1 % x >> y\n>>", b"/A 1 % x >> y\n")]
    #[case(b"<</A <ab>>>", b"/A <ab>")]
    #[case(b"<</A<ab>/B<cd>>>", b"/A<ab>/B<cd>")]
    #[case(b"<</A [<</B (>>)>>]>>", b"/A [<</B (>>)>>]")]
    fn double_angles(#[case] input: &[u8], #[case] expected: &[u8]) {
        let mut input = BStr::new(input);
        let DoubleAngles(inside) = extract(&mut input).unwrap();

        assert_eq!(inside, expected);
        assert!(input.is_empty());
    }

    /// The scan stops at the closing delimiter and leaves what follows.
    #[rstest]
    #[case(
        b"[(plane)-333([)]TJ /F49 9.9626 Tf [(Colomb)51(o)]TJ",
        b"TJ /F49 9.9626 Tf [(Colomb)51(o)]TJ"
    )]
    #[case(b"[1 2] ] 3", b" ] 3")]
    fn brackets_leave_the_rest(#[case] input: &[u8], #[case] rest: &[u8]) {
        let mut input = BStr::new(input);
        Brackets::recognize(&mut input).unwrap();

        assert_eq!(input, rest);
    }

    #[rstest]
    // Not the opening delimiter.
    #[case::not_an_array(b"(1 2)")]
    #[case::not_an_array(b"<<1 2>>")]
    // The input ends inside the array, a string, a hexadecimal string or a comment.
    #[case::unterminated(b"[1 2")]
    #[case::unterminated(b"[1 [2]")]
    #[case::unterminated(b"[(a]")]
    #[case::unterminated(b"[(a\\)]")]
    #[case::unterminated(b"[<ab")]
    #[case::unterminated(b"[<</A 1>>")]
    #[case::unterminated(b"[1 % comment ]")]
    // The closing delimiter of another container.
    #[case::wrong_closer(b"[1 2 >>")]
    #[case::wrong_closer(b"[<</A 1 ] >>")]
    #[case::wrong_closer(b"[1 > 2]")]
    fn malformed_brackets(#[case] input: &[u8]) {
        let mut input = BStr::new(input);

        assert!(Brackets::extract(&mut input).is_err());
    }

    #[rstest]
    #[case::not_a_dictionary(b"<ab>")]
    #[case::not_a_dictionary(b"[1 2]")]
    #[case::unterminated(b"<</A 1")]
    #[case::unterminated(b"<</A 1 >")]
    #[case::unterminated(b"<</A (a>>")]
    #[case::unterminated(b"<</A <ab>>")]
    #[case::wrong_closer(b"<</A [1 2 >>")]
    #[case::wrong_closer(b"<</A 1 ] >>")]
    #[case::wrong_closer(b"<</A 1 > /B 2>>")]
    fn malformed_double_angles(#[case] input: &[u8]) {
        let mut input = BStr::new(input);

        assert!(DoubleAngles::extract(&mut input).is_err());
    }

    #[rstest]
    #[case::unterminated(b"(a")]
    #[case::unterminated(b"(a(b)")]
    #[case::unterminated(b"(a\\)")]
    #[case::not_a_string(b"[a]")]
    fn malformed_parentheses(#[case] input: &[u8]) {
        let mut input = BStr::new(input);

        assert!(Parentheses::extract(&mut input).is_err());
    }

    /// `depth` containers nested in each other, arrays and dictionaries alternating.
    fn nested(depth: usize) -> Vec<u8> {
        let mut input = Vec::new();
        for level in 0..depth {
            input.extend_from_slice(if level % 2 == 0 { b"[" } else { b"<</K " });
        }
        for level in (0..depth).rev() {
            input.extend_from_slice(if level % 2 == 0 { b"]" } else { b">>" });
        }
        input
    }

    #[test]
    fn nesting_is_followed_down_to_the_limit() {
        let input = nested(64);
        let mut input = BStr::new(&input);

        Brackets::recognize(&mut input).unwrap();
        assert!(input.is_empty());
    }

    #[test]
    fn nesting_past_the_limit_is_an_error() {
        let input = nested(65);
        let mut input = BStr::new(&input);

        assert!(Brackets::recognize(&mut input).is_err());
    }

    #[rstest]
    #[case(b"<ab> 1", b"<ab>", b" 1")]
    #[case(b"<> 1", b"<>", b" 1")]
    #[case(b"<</A (>)>> 1", b"<</A (>)>>", b" 1")]
    #[case(b"<</A <ab>>>>", b"<</A <ab>>>", b">")]
    fn dictionary_or_hexadecimal_string(
        #[case] input: &[u8],
        #[case] expected: &[u8],
        #[case] rest: &[u8],
    ) {
        let mut input = BStr::new(input);

        assert_eq!(recognize_angled(&mut input).unwrap(), expected);
        assert_eq!(input, rest);
    }

    #[rstest]
    #[case(b"<ab")]
    #[case(b"<</A 1>")]
    #[case(b"[1]")]
    fn malformed_dictionary_or_hexadecimal_string(#[case] input: &[u8]) {
        let mut input = BStr::new(input);

        assert!(recognize_angled(&mut input).is_err());
    }

    #[rstest]
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
