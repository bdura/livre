use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::digit1,
    combinator::verify,
    multi::many0,
    sequence::{separated_pair, Tuple},
    IResult,
};

use crate::{
    objects::Reference,
    utilities::{parse_digits, take_whitespace, take_whitespace1},
};

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

/// Mapping between indirect objects and the byte offset.
#[derive(Debug, Clone, Default)]
pub struct CrossRefs(pub HashMap<Reference, usize>);

impl CrossRef {
    fn parse(input: &[u8]) -> IResult<&[u8], Self> {
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

    fn into_ref_offset(self, object: usize) -> (Reference, usize) {
        let Self {
            offset,
            gen: generation,
            ..
        } = self;
        let reference = Reference { object, generation };
        (reference, offset)
    }
}

impl CrossRefs {
    fn parse_subsection(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, (start, len)) =
            separated_pair(parse_digits::<usize, _>, tag(b" "), parse_digits)(input)?;

        let (input, _) = take_whitespace1(input)?;

        let (input, refs) = many0(CrossRef::parse)(input)?;

        let (input, _) = take_whitespace(input)?;

        assert_eq!(refs.len(), len);

        let map = refs
            .into_iter()
            .enumerate()
            .map(|(i, r)| r.into_ref_offset(start + i))
            .collect();

        Ok((input, Self(map)))
    }

    /// Parse a cross-reference section.
    pub fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, _) = tag("xref")(input)?;
        let (input, _) = take_whitespace1(input)?;

        let (input, refs) = many0(Self::parse_subsection)(input)?;

        let refs = refs
            .into_iter()
            .fold(CrossRefs::default(), |a, b| a.merge(b));

        Ok((input, refs))
    }
}

impl CrossRefs {
    /// Merge two [`CrossRefs`] objects together.
    ///
    /// Previous updates (the argument) should be overwritten.
    pub fn merge(self, older: Self) -> Self {
        // By chaining `self` _after_ `older`, we make sure that values from `self` will be kept,
        let map = older.0.into_iter().chain(self.0).collect();
        Self(map)
    }
}

impl Deref for CrossRefs {
    type Target = HashMap<Reference, usize>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for CrossRefs {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use indoc::indoc;
    use rstest::rstest;

    fn parse_crossref(input: &[u8]) -> CrossRef {
        let (_, obj) = CrossRef::parse(input).unwrap();
        obj
    }

    #[rstest]
    #[case(b"0000000000 65535 f \n", CrossRef { offset: 0, gen: 65535, used: false })]
    #[case(b"0000000001 65535 f \r", CrossRef { offset: 1, gen: 65535, used: false })]
    #[case(b"0000000002 00000 n\r\n", CrossRef { offset: 2, gen: 0, used: true })]
    fn cross_ref(#[case] input: &[u8], #[case] expected: CrossRef) {
        assert_eq!(parse_crossref(input), expected);
    }

    #[rstest]
    #[case(b"0000000000 65535 f\n")]
    #[case(b"0000000001 6553 f \r")]
    #[case(b"000000002 00000 n\r\n")]
    #[case(b"0000000002  0000 n\r\n")]
    #[should_panic]
    fn failure_cases(#[case] input: &[u8]) {
        parse_crossref(input);
    }

    #[test]
    fn cross_refs() {
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
            0000025777 00000 n \r
        "};

        let (input, refs) = CrossRefs::parse(input).unwrap();

        assert!(input.is_empty());
        assert_eq!(refs.len(), 5);

        assert_eq!(refs.get(&Reference::new(10, 10)), None);

        let expected = vec![
            (Reference::new(0, 65535), 0),
            (Reference::new(3, 0), 25325),
            (Reference::new(23, 2), 25518),
            (Reference::new(24, 0), 25635),
            (Reference::new(30, 0), 25777),
        ];

        for (r, o) in &expected {
            assert_eq!(refs.get(r), Some(o));
        }

        let mut result: Vec<(Reference, usize)> = refs.0.into_iter().collect();
        result.sort_by_key(|(_, o)| *o);

        assert_eq!(expected, result);
    }
}
