use nom::IResult;

pub trait Extract<'input>: Sized {
    fn extract(input: &'input [u8]) -> IResult<&'input [u8], Self>;
}

mod parse;
pub use parse::Parse;

mod boolean;
mod numbers;
mod string;
mod vec;

mod dictionary;
pub use dictionary::Dictionary;
