use nom::{
    bytes::complete::{tag, take_until},
    sequence::separated_pair,
};

use livre_extraction::{
    utilities::{take_whitespace, take_whitespace1},
    Extract,
};

use crate::simple::Reference;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Raw<'input>(pub &'input [u8]);

const ENDOBJ_TAG: &[u8] = b"endobj";

impl<'input> Extract<'input> for Raw<'input> {
    fn extract(input: &'input [u8]) -> nom::IResult<&'input [u8], Self> {
        let (input, raw) = take_until(ENDOBJ_TAG)(input)?;
        Ok((input, Self(raw)))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Indirect<T> {
    pub reference: Reference,
    pub inner: T,
}

impl<'input, T> Extract<'input> for Indirect<T>
where
    T: Extract<'input>,
{
    fn extract(input: &'input [u8]) -> nom::IResult<&'input [u8], Self> {
        let (input, (object, generation)) =
            separated_pair(usize::extract, tag(" "), u16::extract)(input)?;

        let (input, _) = take_whitespace1(input)?;
        let (input, _) = tag(b"obj")(input)?;
        let (input, _) = take_whitespace(input)?;

        let (input, inner) = T::extract(input)?;

        let (input, _) = take_whitespace(input)?;
        let (input, _) = tag(b"endobj")(input)?;

        let reference = Reference { object, generation };

        let indirect = Self { reference, inner };

        Ok((input, indirect))
    }
}

pub type RawIndirect<'i> = Indirect<Raw<'i>>;

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(b"0 0 obj\n1\nendobj", Reference::new(0, 0), 1)]
    #[case(b"202 10 obj\n  -1384\nendobj", Reference::new(202, 10), -1384)]
    fn indirect_i32(#[case] input: &[u8], #[case] reference: Reference, #[case] expected: i32) {
        let (_, indirect) = Indirect::<i32>::extract(input).unwrap();
        assert_eq!(indirect.inner, expected);
        assert_eq!(indirect.reference, reference);
    }

    #[rstest]
    #[case(b"0 0 obj\n1\nendobj", Reference::new(0, 0), b"1\n")]
    #[case(b"202 10 obj\n  -1384\nendobj", Reference::new(202, 10), b"-1384\n")]
    fn indirect_raw(#[case] input: &[u8], #[case] reference: Reference, #[case] expected: &[u8]) {
        let (_, indirect) = Indirect::<Raw>::extract(input).unwrap();
        assert_eq!(indirect.inner, Raw(expected));
        assert_eq!(indirect.reference, reference);
    }
}
