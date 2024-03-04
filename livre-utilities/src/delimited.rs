use nom::{
    error::{Error, ErrorKind, ParseError},
    Err, IResult,
};

/// Consume the inside of brackets until it is unbalanced.
///
/// Adapted from <https://stackoverflow.com/questions/70630556/parse-allowing-nested-parentheses-in-nom>
pub fn take_within_balanced(
    opening_bracket: u8,
    closing_bracket: u8,
) -> impl Fn(&[u8]) -> IResult<&[u8], &[u8]> {
    move |input: &[u8]| {
        if input.is_empty() || input[0] != opening_bracket {
            return Err(Err::Error(Error::from_error_kind(
                input,
                ErrorKind::TakeUntil,
            )));
        }

        let mut bracket_counter = 1;

        for (i, &b) in input.iter().enumerate().skip(1) {
            if b == opening_bracket {
                bracket_counter += 1;
            } else if b == closing_bracket {
                bracket_counter -= 1;
            }

            if bracket_counter == 0 {
                return Ok((&input[i + 1..], &input[1..i]));
            }
        }

        Err(Err::Error(Error::from_error_kind(
            input,
            ErrorKind::TakeUntil,
        )))
    }
}

pub fn parse_dict_body(input: &[u8]) -> IResult<&[u8], &[u8]> {
    // dictionaries are enclosed by double angle brackets.
    let (input, value) = take_within_balanced(b'<', b'>')(input)?;
    let (d, value) = take_within_balanced(b'<', b'>')(value)?;

    if !d.is_empty() {
        return Err(Err::Error(Error::from_error_kind(
            input,
            ErrorKind::TakeTill1,
        )));
    }

    Ok((input, value))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(b"<test>", b"test")]
    #[case(b"<>", b"")]
    #[case(b"<te<s>t>", b"te<s>t")]
    #[case(b"<te<s>eafwt>", b"te<s>eafwt")]
    fn chevron_delimited(#[case] input: &[u8], #[case] expected: &[u8]) {
        let (_, inside) = take_within_balanced(b'<', b'>')(input).unwrap();
        assert_eq!(inside, expected);
    }
}
