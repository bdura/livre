use winnow::{combinator::trace, BStr, PResult, Parser};

use crate::Extract;

impl Extract<'_> for () {
    fn extract(input: &mut &BStr) -> PResult<Self> {
        trace("livre-unit", b"null".value(())).parse_next(input)
    }
}

#[cfg(test)]
mod tests {
    use crate::extraction::extract;

    #[test]
    fn unit() {
        assert_eq!((), extract(&mut b"null".as_ref().into()).unwrap());
    }
}