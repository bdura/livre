use nom::{combinator::recognize, IResult};

/// Extraction trait
pub trait Extract<'input>: Sized {
    fn extract(input: &'input [u8]) -> IResult<&'input [u8], Self>;

    // Useful to implement lighter parsing, eg digits.
    fn recognize(input: &'input [u8]) -> IResult<&'input [u8], &'input [u8]> {
        recognize(Self::extract)(input)
    }
}
