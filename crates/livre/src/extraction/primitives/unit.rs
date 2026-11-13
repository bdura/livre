use winnow::{combinator::trace, BStr, ModalResult, Parser};

use crate::extraction::Extract;

impl Extract<'_> for () {
    fn extract(input: &mut &BStr) -> ModalResult<Self> {
        trace("livre-unit", b"null".value(())).parse_next(input)
    }
}

#[cfg(test)]
mod tests {
    use crate::extraction::extract;

    #[test]
    fn unit() {
        extract::<()>(&mut b"null".as_ref().into()).unwrap();
    }
}
