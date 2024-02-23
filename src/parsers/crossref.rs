use std::collections::HashMap;

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::digit1,
    combinator::verify,
    multi::{many0, separated_list0},
    sequence::{separated_pair, Tuple},
    IResult,
};

use crate::parsers::utilities::{parse_digits, take_eol, take_whitespace};

use super::utilities::take_whitespace1;

/// Cross-reference entry EOL.
/// Can be: SP CR, SP LF, or CR LF (OMG!)
fn xref_entry_eol(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((tag(b" \r"), tag(b" \n"), tag(b"\r\n")))(input)
}

#[derive(Debug, PartialEq, Clone)]
pub struct CrossRef {
    offset: usize,
    gen: u16,
    used: bool,
}

impl CrossRef {
    pub fn parse(input: &[u8]) -> IResult<&[u8], HashMap<usize, Self>> {
        let (input, _) = tag("xref")(input)?;
        let (input, _) = take_whitespace1(input)?;

        let (input, refs) = many0(Self::parse_sub_section)(input)?;

        let refs = refs.into_iter().flatten().collect();

        Ok((input, refs))
    }

    fn parse_sub_section(input: &[u8]) -> IResult<&[u8], Vec<(usize, Self)>> {
        let (input, (start, len)) =
            separated_pair(parse_digits::<usize, _>, tag(b" "), parse_digits)(input)?;

        let (input, _) = take_whitespace1(input)?;

        let (input, refs) = many0(Self::parse_line)(input)?;

        let (input, _) = take_whitespace(input)?;

        assert_eq!(refs.len(), len);

        let map = refs
            .into_iter()
            .enumerate()
            .map(|(i, r)| (i + start, r))
            .collect();

        Ok((input, map))
    }

    fn parse_line(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, (offset, _, gen, _, in_use, _)) = (
            verify(digit1, |r: &[u8]| r.len() == 10),
            tag(b" "),
            verify(digit1, |r: &[u8]| r.len() == 5),
            tag(b" "),
            alt((tag(b"n"), tag(b"f"))),
            xref_entry_eol,
        )
            .parse(input)?;

        // SAFETY: we checked that the bytes are digits, ie UTF-8.
        let offset: usize = unsafe { std::str::from_utf8_unchecked(offset).parse().unwrap() };
        let gen: u16 = unsafe { std::str::from_utf8_unchecked(gen).parse().unwrap() };

        let used = in_use == b"n";

        Ok((input, Self { offset, gen, used }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    #[test]
    fn line() {
        let (_, r) = CrossRef::parse_line(b"0000000000 65535 f \n").unwrap();
        assert_eq!(
            r,
            CrossRef {
                offset: 0,
                gen: 65535,
                used: false
            }
        );

        let (_, r) = CrossRef::parse_line(b"0000025518 00002 n\r\n").unwrap();
        assert_eq!(
            r,
            CrossRef {
                offset: 25518,
                gen: 2,
                used: true
            }
        );

        let (_, r) = CrossRef::parse_line(b"0000025518 00002 n \r").unwrap();
        assert_eq!(
            r,
            CrossRef {
                offset: 25518,
                gen: 2,
                used: true
            }
        );
    }

    #[test]
    fn sub_section() {
        let input = indoc! {b"
            0 2
            0000000000 65535 f 
            0000025325 00000 n\r\n
        "};

        let (input, refs) = CrossRef::parse_sub_section(input).unwrap();
        assert!(input.is_empty());
        assert_eq!(refs.len(), 2);

        let input = indoc! {b"
            23 2
            0000025518 00002 n 
            0000025635 00000 n \r
        "};

        let (input, refs) = CrossRef::parse_sub_section(input).unwrap();
        assert!(input.is_empty());
        assert_eq!(refs.len(), 2);

        assert_eq!(
            refs[0],
            (
                23,
                CrossRef {
                    offset: 25518,
                    gen: 2,
                    used: true
                }
            )
        );

        assert_eq!(
            refs[1],
            (
                24,
                CrossRef {
                    offset: 25635,
                    gen: 0,
                    used: true
                }
            )
        );
    }

    #[test]
    fn cross_ref() {
        let input = indoc! {b"
            xref
            0 1
            0000000000 65535 f 
            3 1
            0000025325 00000 n 
            23 2
            0000025518 00002 n 
            0000025635 00000 n 
            30 1
            0000025777 00000 n \n
        "};

        let (input, refs) = CrossRef::parse(input).unwrap();
        println!("{:?}", String::from_utf8_lossy(input));
        assert!(input.is_empty());
        assert_eq!(refs.len(), 5);
    }
}
