use nom::bytes::complete::tag;

use crate::{
    objects::Name,
    parsers::{extract, take_whitespace, Extract},
    text::TextState,
};

use super::Operator;

macro_rules! ops {
    ($($tag:literal -> $name:ident: $ty:ty)+) => {
        $(
            #[derive(Debug, PartialEq, Clone, Copy)]
            pub struct $name;

            impl Operator for $name {
                fn apply(self, _: &mut TextState) {}
            }

            impl Extract<'_> for $name {
                fn extract(input: &[u8]) -> nom::IResult<&[u8], Self> {
                    let (input, _) = extract::<$ty>(input)?;
                    let (input, _) = take_whitespace(input)?;
                    let (input, _) = tag($tag)(input)?;
                    Ok((input, Self))
                }
            }
        )+
    };
}

ops!(
    b"G" -> UppercaseG: f32
    b"g" -> LowercaseG: f32
    b"gs" -> Gs: Name
    b"rg" -> LowercaseRG: (f32, f32, u8)
    b"RG" -> UppercaseRG: (f32, f32, u8)
);
