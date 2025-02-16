use winnow::{
    combinator::{fail, opt, trace},
    dispatch,
    token::{any, take},
    BStr, PResult, Parser,
};

use crate::extraction::Extract;

/// Standard date used in PDF, with a bespoke format.
#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub struct Date {
    pub year: u16,
    pub month: Option<u8>,
    pub day: Option<u8>,
    pub hour: Option<u8>,
    pub minute: Option<u8>,
    pub second: Option<u8>,
    pub offset: Option<i16>,
}

fn utc_diff_minutes(input: &mut &BStr) -> PResult<i16> {
    let mut minutes = take(2usize)
        .parse_to()
        .map(|n: i16| n * 60)
        .parse_next(input)?;

    b"'".parse_next(input)?;

    if let Some(m) = opt(take(2usize).parse_to::<i16>()).parse_next(input)? {
        minutes += m;
    }

    Ok(minutes)
}

fn utc_diff(input: &mut &BStr) -> PResult<i16> {
    dispatch! {any;
        b'Z' => opt(b"00'".and_then(opt(b"00"))).value(0),
        b'+' => utc_diff_minutes,
        b'-' => utc_diff_minutes.map(|m| -m),
        _ => fail,
    }
    .parse_next(input)
}

fn parse_date_without_offset(input: &mut &BStr) -> PResult<Date> {
    let year = take(4usize).parse_to().parse_next(input)?;

    let mut date = Date {
        year,
        ..Default::default()
    };

    let res = opt(take(2usize).parse_to()).parse_next(input)?;
    if res.is_none() {
        return Ok(date);
    }
    date.month = res;

    let res = opt(take(2usize).parse_to()).parse_next(input)?;
    if res.is_none() {
        return Ok(date);
    }
    date.day = res;

    let res = opt(take(2usize).parse_to()).parse_next(input)?;
    if res.is_none() {
        return Ok(date);
    }
    date.hour = res;

    let res = opt(take(2usize).parse_to()).parse_next(input)?;
    if res.is_none() {
        return Ok(date);
    }
    date.minute = res;

    Ok(date)
}

impl Extract<'_> for Date {
    fn extract(input: &mut &BStr) -> PResult<Self> {
        trace("livre-date", move |i: &mut &BStr| {
            b"D:".parse_next(i)?;
            let mut date = parse_date_without_offset(i)?;
            date.offset = opt(utc_diff).parse_next(i)?;
            Ok(date)
        })
        .parse_next(input)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::extraction::extract;

    use super::*;

    #[rstest]
    #[case(
        b"D:199812231952-08'00",
        Date{
            year: 1998,
            month:Some(12),
            day: Some(23),
            hour: Some(19),
            minute: Some(52),
            second: None,
            offset: Some(-8 * 60),
        },
    )]
    #[case(
        b"D:199812231952+08'00",
        Date{
            year: 1998,
            month:Some(12),
            day: Some(23),
            hour: Some(19),
            minute: Some(52),
            second: None,
            offset: Some(8 * 60),
        },
    )]
    #[case(
        b"D:199812231952Z",
        Date{
            year: 1998,
            month:Some(12),
            day: Some(23),
            hour: Some(19),
            minute: Some(52),
            second: None,
            offset: Some(0),
        },
    )]
    #[case(
        b"D:2000",
        Date{
            year: 2000,
            ..Default::default()
        },
    )]
    #[case(
        b"D:200001",
        Date{
            year: 2000,
            month: Some(1),
            ..Default::default()
        },
    )]
    #[case(
        b"D:20000102",
        Date{
            year: 2000,
            month: Some(1),
            day: Some(2),
            ..Default::default()
        },
    )]
    fn rectangle(#[case] input: &[u8], #[case] expected: Date) {
        let res = extract(&mut input.as_ref()).unwrap();
        assert_eq!(expected, res);
    }
}
