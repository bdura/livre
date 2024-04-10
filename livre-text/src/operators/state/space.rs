use livre_extraction::{extract, Extract};
use livre_utilities::take_whitespace1;
use nom::{
    bytes::complete::tag,
    sequence::{terminated, tuple},
};

use crate::Operator;

macro_rules! space_element {
    ($name:ident + $tag:literal) => {
        #[derive(Debug, PartialEq, Clone, Copy)]
        pub struct $name(pub f32);

        impl Extract<'_> for $name {
            fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
                let (input, space) =
                    terminated(extract, tuple((take_whitespace1, tag($tag))))(input)?;
                Ok((input, Self(space)))
            }
        }
    };
}

space_element!(CharSpace + "Tc");
space_element!(WordSpace + "Tw");
space_element!(Leading + "TL");
space_element!(Rise + "Ts");

/// The specs use a percentage. We transform it to a ratio.
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct HorizontalScale(pub f32);

impl Extract<'_> for HorizontalScale {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        let (input, scale) = f32::extract(input)?;
        let (input, _) = tuple((take_whitespace1, tag("Tz")))(input)?;
        Ok((input, Self(scale / 100.0)))
    }
}

impl Operator for CharSpace {
    fn apply(self, obj: &mut crate::TextState) {
        obj.character_spacing = self.0;
    }
}

impl Operator for WordSpace {
    fn apply(self, obj: &mut crate::TextState) {
        obj.word_spacing = self.0;
    }
}

impl Operator for Leading {
    fn apply(self, obj: &mut crate::TextState) {
        obj.leading = self.0;
    }
}

impl Operator for Rise {
    fn apply(self, obj: &mut crate::TextState) {
        obj.rise = self.0;
    }
}

impl Operator for HorizontalScale {
    fn apply(self, obj: &mut crate::TextState) {
        obj.horizontal_scaling = self.0;
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn create_input(input: &[u8], tag: &[u8]) -> Vec<u8> {
        let mut input = input.to_vec();
        input.push(b' ');
        input.extend(tag);
        input
    }

    macro_rules! test {
        ($name:ident + $tag:literal + $input:ident + $expected:tt) => {
            let _input = create_input($input, $tag);
            let (_, $name(result)) = extract(&_input).unwrap();
            assert_eq!(result, $expected);
        };
    }

    #[rstest]
    #[case(b"12", 12.0)]
    #[case(b"100", 100.0)]
    #[case(b"0.5", 0.5)]
    #[case(b"-0.5", -0.5)]
    fn space_operators(#[case] input: &[u8], #[case] expected: f32) {
        test!(CharSpace + b"Tc" + input + expected);
        test!(WordSpace + b"Tw" + input + expected);
        test!(HorizontalScale + b"Tz" + input + (expected / 100.0));
        test!(Leading + b"TL" + input + expected);
        test!(Rise + b"Ts" + input + expected);
    }
}
