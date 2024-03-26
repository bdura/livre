use std::borrow::Cow;

use nom::{bytes::complete::take_while, IResult};

static CONVERSION_24: &[&[u8]] = &[
    &[0x02, 0xd8], // u BREVE
    &[0x02, 0xC7], // v CARON
    &[0x02, 0xC6], // ^ MODIFIER LETTER CIRCUMFLEX ACCENT
    &[0x02, 0xD9], // · DOT ABOVE
    &[0x02, 0xDD], // ” DOUBLE ACUTE ACCENT
    &[0x02, 0xDB], // , OGONEK
    &[0x02, 0xDA], // ° RING ABOVE
    &[0x02, 0xDC], // ~ SMALL TILDE
];

static CONVERSION_127: &[&[u8]] = &[
    &[],           // Undefined
    &[0x20, 0x22], // •  BULLET
    &[0x20, 0x20], // †  DAGGER
    &[0x20, 0x21], // ‡  DOUBLE DAGGER
    &[0x20, 0x26], // …  HORIZONTAL ELLIPSIS
    &[0x20, 0x14], // —  EM DASH
    &[0x20, 0x13], // –  EN DASH
    &[0x01, 0x92], // ƒ
    &[0x20, 0x44], // ⁄  FRACTION SLASH (solidus)
    &[0x20, 0x39], // ‹  SINGLE LEFT-POINTING ANGLE QUOTATION MARK
    &[0x20, 0x3A], // ›  SINGLE RIGHT-POINTING ANGLE QUOTATION MARK
    &[0x22, 0x12], // Š
    &[0x20, 0x30], // ‰  PER MILLE SIGN
    &[0x20, 0x1E], // „  DOUBLE LOW-9 QUOTATION MARK (quotedblbase)
    &[0x20, 0x1C], // “  LEFT DOUBLE QUOTATION MARK (double quote left)
    &[0x20, 0x1D], // ”  RIGHT DOUBLE QUOTATION MARK (quotedblright)
    &[0x20, 0x18], // ‘  LEFT SINGLE QUOTATION MARK (quoteleft)
    &[0x20, 0x19], // ’  RIGHT SINGLE QUOTATION MARK (quoteright)
    &[0x20, 0x1A], // ‚  SINGLE LOW-9 QUOTATION MARK (quotesinglbase)
    &[0x21, 0x22], // ™  TRADE MARK SIGN
    &[0xFB, 0x01], // fi LATIN SMALL LIGATURE FI
    &[0xFB, 0x02], // fl LATIN SMALL LIGATURE FL
    &[0x01, 0x41], // Ł  LATIN CAPITAL LETTER L WITH STROKE
    &[0x01, 0x52], // Œ  LATIN CAPITAL LIGATURE OE
    &[0x01, 0x60], // Š  LATIN CAPITAL LETTER S WITH CARON
    &[0x01, 0x78], // Ÿ  LATIN CAPITAL LETTER Y WITH DIAERESIS
    &[0x01, 0x7D], // Ž  LATIN CAPITAL LETTER Z WITH CARON
    &[0x01, 0x31], // ı  LATIN SMALL LETTER DOTLESS I
    &[0x01, 0x42], // ł  LATIN SMALL LETTER L WITH STROKE
    &[0x01, 0x53], // œ  LATIN SMALL LIGATURE OE
    &[0x01, 0x61], // š  LATIN SMALL LETTER S WITH CARON
    &[0x01, 0x7E], // ž  LATIN SMALL LETTER Z WITH CARON
    &[],           // Undefined
    &[0x20, 0xAC], // €  EURO SIGN
];

fn is_compatible(b: u8) -> bool {
    match b {
        173 => false,
        val if val < 24 => true,
        val if val < 32 => false,
        val if val < 127 => true,
        val if val < 161 => false,
        _ => true,
    }
}
fn convert(b: u8) -> &'static [u8] {
    match b {
        173 => &[],
        val if val < 32 => CONVERSION_24[val as usize - 24],
        val if val < 161 => CONVERSION_127[val as usize - 127],
        _ => unreachable!("`convert` is covered by `is_compatible`."),
    }
}

fn direct_conversion(input: &[u8]) -> IResult<&[u8], &[u8]> {
    take_while(is_compatible)(input)
}

pub fn pdf_decode(input: &[u8]) -> Cow<'_, [u8]> {
    let (mut new_input, s) = direct_conversion(input).expect("Parser cannot fail");

    if new_input.is_empty() {
        return Cow::Borrowed(s);
    }

    let mut res: Vec<u8> = Vec::with_capacity(input.len());

    res.extend(s);

    while !new_input.is_empty() {
        let (&first, input) = new_input.split_first().expect("New input is not empty");
        res.extend(convert(first));

        let (input, s) = direct_conversion(input).expect("Parser cannot fail");
        res.extend(s);

        new_input = input;
    }

    Cow::Owned(res)
}

static UTF8_MARKER: &[u8] = &[239, 187, 191];
static UTF16BE_MARKER: &[u8] = &[254, 255];

pub fn decode_str(input: &[u8]) -> Cow<'_, str> {
    if input.starts_with(UTF8_MARKER) {
        let input = &input[3..];
        let res = std::str::from_utf8(input).expect("Per the specs, the string is UTF-8 encoded");
        Cow::Borrowed(res)
    } else if input.starts_with(UTF16BE_MARKER) {
        let input = &input[2..];
        let utf16: Vec<u16> = input
            .chunks_exact(2)
            .map(|b| u16::from_be_bytes([b[0], b[1]]))
            .collect();
        let res =
            String::from_utf16(&utf16).expect("Per the specs, the string is UTF-16BE encoded");
        Cow::Owned(res)
    } else {
        let utf8 = pdf_decode(input);
        match utf8 {
            Cow::Borrowed(input) => {
                let res =
                    std::str::from_utf8(input).expect("Per the specs, the string is UTF-8 encoded");
                Cow::Borrowed(res)
            }
            Cow::Owned(input) => {
                let res = std::str::from_utf8(&input)
                    .expect("Per the specs, the string is UTF-8 encoded");
                Cow::Owned(res.to_owned())
            }
        }
    }
}
