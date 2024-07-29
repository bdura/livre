use std::fmt::Debug;

use nom::{multi::count, sequence::terminated};

use super::{extract, extraction::Extract, take_whitespace, Brackets};

impl<'input, T, const S: usize> Extract<'input> for [T; S]
where
    T: Extract<'input> + Debug,
{
    fn extract(input: &'input [u8]) -> nom::IResult<&'input [u8], Self> {
        let (input, Brackets(value)) = extract(input)?;

        // We need to remove preceding whitespace.
        let (value, _) = take_whitespace(value)?;

        let (value, array) = count(terminated(T::extract, take_whitespace), S)(value)?;
        let (value, _) = take_whitespace(value)?;

        assert!(
            value.is_empty(),
            "Remainder is not empty: {:?}",
            String::from_utf8_lossy(value)
        );

        let array = array
            .try_into()
            .expect("By construction, the type and size are correct.");

        Ok((input, array))
    }
}

#[cfg(test)]
mod tests {

    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(b"[1 2 4]", [1, 2, 4])]
    #[case(b"[  1  2 4   ]", [1, 2, 4])]
    fn array_i32(#[case] input: &[u8], #[case] expected: [i32; 3]) {
        let (_, parsed) = extract::<[i32; 3]>(input).unwrap();
        assert_eq!(expected, parsed);
    }
}
