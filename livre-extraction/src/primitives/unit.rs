use nom::bytes::complete::tag;

use crate::extraction::Extract;

impl Extract<'_> for () {
    fn extract(input: &[u8]) -> nom::IResult<&[u8], Self> {
        let (input, _) = tag(b"null")(input)?;
        Ok((input, ()))
    }
}

#[cfg(test)]
mod tests {
    use crate::parse;

    #[test]
    fn extract_unit() {
        parse::<()>(b"null").unwrap();
    }
}
