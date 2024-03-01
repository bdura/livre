use livre_extraction::{Extract, Reference};
use livre_utilities::{space, take_whitespace, take_whitespace1};
use nom::{
    branch::alt,
    bytes::complete::tag,
    combinator::map,
    multi::many0,
    sequence::{separated_pair, tuple},
    IResult,
};

/// Cross-reference entry EOL.
/// Can be: SP CR, SP LF, or CR LF (OMG!)
fn xref_entry_eol(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((tag(" \r"), tag(" \n"), tag("\r\n")))(input)
}

#[derive(Debug, PartialEq, Clone)]
struct CrossRef {
    offset: usize,
    generation: u16,
    used: bool,
}

impl Extract<'_> for CrossRef {
    fn extract(input: &[u8]) -> IResult<&[u8], Self> {
        // We do not check the padding length... Should be fine, right?
        let (input, (offset, _, generation, _, used, _)) = tuple((
            usize::extract,
            space,
            u16::extract,
            space,
            map(alt((tag(b"n"), tag(b"f"))), |b| b == b"n"),
            xref_entry_eol,
        ))(input)?;

        let crossref = Self {
            offset,
            generation,
            used,
        };

        Ok((input, crossref))
    }
}

impl CrossRef {
    fn into_ref_offset(self, object: usize) -> (Reference, usize) {
        let Self {
            offset, generation, ..
        } = self;

        let reference = Reference::new(object, generation);

        (reference, offset)
    }
}

fn parse_xref_section(input: &[u8]) -> IResult<&[u8], Vec<(Reference, usize)>> {
    let (input, (start, len)) = separated_pair(usize::extract, space, usize::extract)(input)?;

    let (input, _) = take_whitespace1(input)?;

    let (input, refs) = many0(CrossRef::extract)(input)?;

    let (input, _) = take_whitespace(input)?;

    assert_eq!(refs.len(), len);

    let res = refs
        .into_iter()
        .enumerate()
        .map(|(i, r)| r.into_ref_offset(start + i))
        .collect();

    Ok((input, res))
}

pub(super) struct PlainCrossRefs(pub Vec<(Reference, usize)>);

impl Extract<'_> for PlainCrossRefs {
    fn extract(input: &'_ [u8]) -> IResult<&'_ [u8], Self> {
        let (input, _) = tag("xref")(input)?;
        let (input, _) = take_whitespace1(input)?;

        let (input, refs) = many0(parse_xref_section)(input)?;

        let refs = refs.into_iter().flatten().collect();

        Ok((input, Self(refs)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use indoc::indoc;
    use rstest::rstest;

    fn parse(input: &[u8]) -> CrossRef {
        let (_, obj) = CrossRef::extract(input).unwrap();
        obj
    }

    #[rstest]
    #[case(b"0000000000 65535 f \n", CrossRef { offset: 0, generation: 65535, used: false })]
    #[case(b"0000000001 65535 f \r", CrossRef { offset: 1, generation: 65535, used: false })]
    #[case(b"0000000002 00000 n\r\n", CrossRef { offset: 2, generation: 0, used: true })]
    fn crossref(#[case] input: &[u8], #[case] expected: CrossRef) {
        assert_eq!(parse(input), expected);
    }

    #[rstest]
    #[case(b"0000000000 65535 f\n")]
    #[case(b"0000000001 65536 f \r")]
    #[should_panic]
    fn failure_cases(#[case] input: &[u8]) {
        parse(input);
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

        let (input, PlainCrossRefs(refs)) = PlainCrossRefs::extract(input).unwrap();

        assert!(input.is_empty());
        assert_eq!(refs.len(), 5);

        let expected = vec![
            (Reference::new(0, 65535), 0),
            (Reference::new(3, 0), 25325),
            (Reference::new(23, 2), 25518),
            (Reference::new(24, 0), 25635),
            (Reference::new(30, 0), 25777),
        ];

        assert_eq!(expected, refs);
    }
}
