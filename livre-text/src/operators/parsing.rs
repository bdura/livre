use livre_extraction::{extract, Brackets, Extract};
use livre_utilities::{is_space_or_newline, take_whitespace, take_whitespace1};
use nom::{branch::alt, bytes::complete::take_till, combinator::map, IResult};

use super::{CharSpace, ElementsExtract, HorizontalScale, Op, WordSpace};

struct Element<'i>(&'i [u8]);

impl<'i> Extract<'i> for Element<'i> {
    fn extract(input: &'i [u8]) -> nom::IResult<&'i [u8], Self> {
        map(alt((parse_array, parse_element)), Element)(input)
    }
}

fn parse_array(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (input, Brackets(out)) = extract(input)?;
    let (input, _) = take_whitespace1(input)?;

    Ok((input, out))
}

fn parse_element(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (input, out) = take_till(is_space_or_newline)(input)?;
    let (input, _) = take_whitespace(input)?;

    Ok((input, out))
}

struct IterOperators<'i> {
    elements: Vec<&'i [u8]>,
    start: isize,
    stop: isize,
}

impl Iterator for IterOperators<'_> {
    type Item = Op;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

fn is_op(current: &[u8], elements: &[&[u8]]) -> Option<Op> {
    match current {
        b"Tc" => Some(Op::from(CharSpace::extract_from_elements(elements))),
        b"Tw" => Some(Op::from(WordSpace::extract_from_elements(elements))),
        b"Th" => Some(Op::from(HorizontalScale::extract_from_elements(elements))),
        _ => todo!(),
    }
}
