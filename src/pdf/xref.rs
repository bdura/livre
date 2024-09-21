use std::{fmt::Debug, str::FromStr};

use winnow::{
    ascii::{line_ending, multispace0, multispace1},
    combinator::{alt, delimited, iterator, repeat, separated_pair, terminated, trace},
    error::ContextError,
    token::{take_until, take_while},
    BStr, PResult, Parser,
};

use crate::extraction::{extract, Extract, ReferenceId};

#[derive(Debug, Clone, Copy)]
pub struct StartXRef(pub usize);

impl Extract<'_> for StartXRef {
    fn extract(input: &mut &BStr) -> PResult<Self> {
        let (_, _, value) = (b"startxref", multispace1, extract).parse_next(input)?;
        Ok(Self(value))
    }
}

impl StartXRef {
    pub fn find(input: &BStr) -> PResult<Self> {
        let mut i = &input[(input.len().saturating_sub(30))..];
        take_until(0.., b"startxref".as_slice()).parse_next(&mut i)?;
        Self::extract(&mut i)
    }
}

fn dec_num<'de, T, E>(count: usize) -> impl Parser<&'de BStr, T, ContextError>
where
    T: FromStr<Err = E>,
    E: Debug,
{
    move |input: &mut &'de BStr| {
        let num = take_while(count, b'0'..=b'9').parse_next(input)?;
        let num = unsafe { std::str::from_utf8_unchecked(num) };
        let n = num.parse().expect("correct by construction");

        Ok(n)
    }
}

fn xref_entry(input: &mut &BStr) -> PResult<(usize, u16, bool)> {
    trace("livre-ref-entry", move |i: &mut &BStr| {
        let (offset, gen) = separated_pair(dec_num(10), b' ', dec_num(5)).parse_next(i)?;

        let in_use = delimited(
            b' ',
            alt((b'n'.value(true), b'f'.value(false))),
            alt((b" \n", b" \r", b"\r\n")),
        )
        .parse_next(i)?;

        Ok((offset, gen, in_use))
    })
    .parse_next(input)
}

fn xref_subsection<'de>(input: &mut &'de BStr) -> PResult<Vec<(ReferenceId, usize)>> {
    let (initial, n) = separated_pair(usize::extract, b' ', usize::extract).parse_next(input)?;
    line_ending(input)?;

    let entries: Vec<(usize, u16, bool)> = repeat(n, xref_entry).parse_next(input)?;

    let res = entries
        .into_iter()
        .enumerate()
        .filter_map(|(i, (offset, gen, in_use))| {
            if !in_use {
                return None;
            }
            Some((ReferenceId::new(initial + i, gen), offset))
        })
        .collect();

    Ok(res)
}

fn xref(input: &mut &BStr) -> PResult<Vec<(ReferenceId, usize)>> {
    (b"xref", multispace1).parse_next(input)?;

    let mut it = iterator(*input, terminated(xref_subsection, multispace0));
    let res = it.flatten().collect();
    *input = it.finish()?.0;

    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;

    use indoc::indoc;
    use rstest::rstest;

    #[rstest]
    #[case(b"0000000000 65535 f \n", (0, 65535, false))]
    #[case(b"0000000010 00000 n\r\n", (10, 0, true))]
    #[case(b"0000000103 00001 n \r", (103, 1, true))]
    fn extract_ref_entry(#[case] input: &[u8], #[case] expected: (usize, u16, bool)) {
        let result = xref_entry(&mut input.as_ref()).unwrap();
        assert_eq!(expected, result)
    }

    #[rstest]
    #[case(
        indoc!{b"
            0 1
            0000000000 65535 f 
        "},
        Vec::new()
    )]
    #[case(
        indoc!{b"
            10 2
            0000000001 00001 f 
            0000000035 00000 n 
        "},
        vec![
            ((11, 0).into(), 35),
        ]
    )]
    #[case(
        indoc!{b"
            110 3
            0000010001 00020 n 
            0000010020 00000 n 
            0000050020 03482 n 
        "},
        vec![
            ((110, 20).into(), 10001),
            ((111, 0).into(), 10020),
            ((112, 3482).into(), 50020),
        ]
    )]
    fn subsection(#[case] input: &[u8], #[case] expected: Vec<(ReferenceId, usize)>) {
        let res = xref_subsection(&mut input.as_ref()).unwrap();
        assert_eq!(expected, res)
    }

    #[rstest]
    #[case(
        indoc!{b"
            xref
            0 3
            0000000000 65535 f 
            0000000200 00000 n\r
            0000000220 00001 n \r
            3 2
            0000000300 00001 f 
            0000000300 00000 n\r
        "},
        vec![
            ((1, 0).into(), 200),
            ((2, 1).into(), 220),
            ((4, 0).into(), 300),
        ]
    )]
    fn xref_extraction(#[case] input: &[u8], #[case] expected: Vec<(ReferenceId, usize)>) {
        let res = xref(&mut input.as_ref()).unwrap();
        assert_eq!(expected, res)
    }

    #[test]
    fn startxref() {
        let input = indoc! {b"
            test
            test
            test
            test
            test
            test
            test
            test
            test
            test
            startxref
            7
        "}
        .as_slice();

        assert_eq!(input.len(), 62);

        let StartXRef(value) = StartXRef::find(input.as_ref()).unwrap();

        assert_eq!(input.len(), 62);
        assert_eq!(value, 7);
    }
}
