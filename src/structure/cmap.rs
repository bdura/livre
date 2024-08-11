use std::collections::BTreeMap;

use crate::parsers::Extract;
use crate::parsers::{extract, take_whitespace, HexU16};
use nom::branch::alt;
use nom::bytes::complete::{tag, take, take_until};
use nom::combinator::map;
use nom::multi::separated_list0;
use nom::sequence::preceded;
use nom::IResult;
use nom::{multi::count, sequence::tuple};

use crate::objects::Stream;

// #[derive(Debug, Deserialize)]
// #[serde(untagged)]
// pub enum NameOrStream {
//     Name(String),
//     Stream(OptRef<CMap>),
// }

// #[derive(Debug, Deserialize)]
// #[serde(rename_all = "PascalCase")]
// struct CMapConfig {
//     use_c_map: Option<Object>,
// }

#[derive(Debug)]
pub struct CMap<I>(pub I);

impl<'input, I> Extract<'input> for CMap<I>
where
    I: for<'a> Extract<'a>,
{
    fn extract(input: &'input [u8]) -> nom::IResult<&'input [u8], Self> {
        let (
            input,
            Stream {
                decoded,
                structured: (),
            },
        ) = Stream::extract(input)?;

        let (_, inner) = extract(&decoded).unwrap();

        Ok((input, Self(inner)))
    }
}

#[derive(Debug, PartialEq)]
pub struct InnerToUnicode(pub BTreeMap<u16, Vec<u16>>);

impl Extract<'_> for InnerToUnicode {
    fn extract(input: &'_ [u8]) -> IResult<&'_ [u8], Self> {
        let (input, _) = take_until("endcodespacerange")(input)?;
        let (input, _) = take(17usize)(input)?;
        let (input, _) = take_whitespace(input)?;

        let (input, mapping) = separated_list0(take_whitespace, take_bfrange_or_bfchar)(input)?;

        let btree = mapping.into_iter().flatten().collect();

        Ok((input, Self(btree)))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ToUnicode(pub BTreeMap<u16, Vec<u16>>);

impl Extract<'_> for ToUnicode {
    fn extract(input: &'_ [u8]) -> IResult<&'_ [u8], Self> {
        let (input, CMap(InnerToUnicode(btree))) = extract(input)?;
        Ok((input, Self(btree)))
    }
}

fn take_bfrange_or_bfchar(input: &[u8]) -> IResult<&[u8], Vec<(u16, Vec<u16>)>> {
    alt((
        map(extract, |BfRange(value)| value),
        map(extract, |BfChar(value)| value),
    ))(input)
}

#[derive(Debug, PartialEq)]
pub struct BfRange(pub Vec<(u16, Vec<u16>)>);

impl Extract<'_> for BfRange {
    fn extract(input: &'_ [u8]) -> IResult<&'_ [u8], Self> {
        let (input, n) = extract(input)?;
        let (input, _) = tuple((take_whitespace, tag(b"beginbfrange")))(input)?;
        let (input, lines) = count(preceded(take_whitespace, extract_bfrange_line), n)(input)?;
        let lines = lines.into_iter().flatten().collect();
        let (input, _) = tuple((take_whitespace, tag(b"endbfrange")))(input)?;
        Ok((input, Self(lines)))
    }
}

#[derive(Debug, PartialEq)]
pub struct BfChar(pub Vec<(u16, Vec<u16>)>);

impl Extract<'_> for BfChar {
    fn extract(input: &'_ [u8]) -> IResult<&'_ [u8], Self> {
        let (input, n) = extract(input)?;
        let (input, _) = tuple((take_whitespace, tag(b"beginbfchar")))(input)?;
        let (input, lines) = count(preceded(take_whitespace, extract_bchar_line), n)(input)?;
        let (input, _) = tuple((take_whitespace, tag(b"endbfchar")))(input)?;
        Ok((input, Self(lines)))
    }
}

#[derive(Debug, PartialEq)]
struct SingleU16(u16);

impl Extract<'_> for SingleU16 {
    fn extract(input: &'_ [u8]) -> IResult<&'_ [u8], Self> {
        let (input, HexU16(mut vec)) = extract(input)?;
        let value = vec.pop().unwrap();
        Ok((input, Self(value)))
    }
}

fn extract_bfrange_line(input: &[u8]) -> IResult<&[u8], Vec<(u16, Vec<u16>)>> {
    let (input, (SingleU16(first), SingleU16(last))) = extract(input)?;
    let (input, _) = take_whitespace(input)?;

    if let Ok((input, SingleU16(value))) = extract(input) {
        let result = (0..=(last - first))
            .map(|i| (first + i, vec![value + i]))
            .collect();
        Ok((input, result))
    } else {
        let (input, elements) = extract::<Vec<HexU16>>(input)?;
        let result = (first..)
            .zip(elements.into_iter().map(|HexU16(value)| value))
            .collect();
        Ok((input, result))
    }
}

fn extract_bchar_line(input: &[u8]) -> IResult<&[u8], (u16, Vec<u16>)> {
    let (input, (SingleU16(codepoint), HexU16(value))) = extract(input)?;
    Ok((input, (codepoint, value)))
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use indoc::indoc;
    use rstest::rstest;

    use crate::{objects::Bytes, parsers::parse};

    use super::*;

    #[rstest]
    #[case(b"<0066>", SingleU16(0x0066))]
    #[case(
        indoc!{b"
            2 beginbfrange
            <0000> <0001> <0020>
            <005F> <0060> [<00660066> <00660066006C>]
            endbfrange
        "},
        BfRange(vec![
            (0x0000, vec![0x0020]),
            (0x0001, vec![0x0021]),
            (0x005F, vec![0x0066, 0x0066]),
            (0x0060, vec![0x0066, 0x0066, 0x006C]),
        ])
    )]
    #[case(
        indoc!{b"
            1 beginbfchar
            <3A51> <D840DC3E>
            endbfchar
        "},
        BfChar(vec![
            (0x3A51, vec![0xD840, 0xDC3E]),
        ])
    )]
    #[should_panic]
    #[case(
        indoc!{b"
            1 beginbfchar
            <0001> <D840DC3E0066>
            <3A51> <D840DC3E>
            endbfchar
        "},
        BfChar(vec![
            (0x3A51, vec![0xD840, 0xDC3E]),
        ])
    )]
    #[case(
        indoc!{b"
        2 beginbfchar
        <0001> <D840DC3E0066>
        <3A51> <D840DC3E>
        endbfchar
        "},
        BfChar(vec![
            (0x0001, vec![0xD840, 0xDC3E, 0x0066]),
            (0x3A51, vec![0xD840, 0xDC3E]),
        ])
    )]
    fn extract<'a, T>(#[case] input: &'a [u8], #[case] expected: T)
    where
        T: Extract<'a> + Debug + PartialEq,
    {
        let result = parse(input).unwrap();
        assert_eq!(expected, result);
    }

    #[rstest]
    #[case(
        indoc!{b"
            <<
            /Type /CMap
            /CMapName /Adobe-Identity-UCS2
            /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS2) /Supplement 0 >>
            /Length 433
            >>
            stream
            /CIDInit /ProcSet findresource begin
            12 dict begin
            begincmap
            /CIDSystemInfo
            <</Registry (Adobe)
            /Ordering (UCS2)
            /Supplement 0
            >> def
            /CMapName /Adobe-Identity-UCS2 def
            /CMapType 2 def
            1 begincodespacerange
            <0000> <FFFF>
            endcodespacerange
            2 beginbfrange
            <0000> <0001> <0020>
            <005F> <0060> [<00660066> <00660066006C>]
            endbfrange
            1 beginbfchar
            <3A51> <D840DC3E>
            endbfchar
            endcmap
            CMapName currentdict /CMap defineresource pop
            end
            end

            endstream
        "},
        vec![
            (0x0000, vec![0x0020]),
            (0x0001, vec![0x0021]),
            (0x005F, vec![0x0066, 0x0066]),
            (0x0060, vec![0x0066, 0x0066, 0x006C]),
            (0x3A51, vec![0xD840, 0xDC3E]),
        ]
    )]
    fn to_unicode(#[case] input: &[u8], #[case] expected: Vec<(u16, Vec<u16>)>) {
        let (_, ToUnicode(btree)) = ToUnicode::extract(input)
            .map_err(|e| e.map_input(Bytes::from))
            .unwrap();
        assert_eq!(btree, expected.into_iter().collect())
    }
}
