use livre_extraction::Extract;
use nom::{branch::alt, bytes::complete::tag, combinator::map};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Variant {
    Page,
    Template,
}

impl Extract<'_> for Variant {
    fn extract(input: &'_ [u8]) -> nom::IResult<&'_ [u8], Self> {
        alt((
            map(tag("/Page"), |_| Variant::Page),
            map(tag("/Template"), |_| Variant::Template),
        ))(input)
    }
}

#[cfg(test)]
mod tests {

    use livre_extraction::extract;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(b"/Page", Variant::Page)]
    #[case(b"/Template", Variant::Template)]
    fn page(#[case] input: &[u8], #[case] expected: Variant) {
        let (_, variant) = extract(input).unwrap();
        assert_eq!(expected, variant);
    }
}
